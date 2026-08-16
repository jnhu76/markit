//! Markit Phase A0 — GPUI Windows feasibility prototype.
//!
//! Thin editable text surface: window, fixed-font text (incl. Chinese via
//! DirectWrite fallback), mouse/keyboard input, insert/delete, cursor,
//! selection, scroll, resize, HiDPI baseline, IME (via gpui's input-handler
//! path, IMM32 on Windows).
//!
//! Controls: type to insert, arrows/backspace/delete/enter/home/end,
//! shift+arrows to select, ctrl+a/c/v/x, wheel to scroll, F1 dumps the
//! instrumentation ring buffer to stdout, ctrl+q quits.
//!
//! `--smoke`: deterministic self-test — drives the same input-handler path the
//! platform uses (plain chars, IME composition/commit, keystroke actions,
//! scroll, resize), prints editor state per step, then dumps the trace and
//! exits. Used to verify the pipeline without a human at the keyboard.

mod editor;
mod instrument;

use gpui::{
    prelude::*, App, Application, Bounds, Entity, Keystroke, KeyBinding, Window, WindowBounds,
    WindowOptions, px, size,
};

use editor::ThinEditor;
use instrument::Stage;

// NOTE (Phase A0 finding): `frame_submit` is marked unavailable on GPUI
// 0.2.2 Windows. `Window::on_next_frame` callbacks never fired in this spike
// (registered in the window-open update; draws did occur — layout/render
// stages were recorded every frame), and the moment between the last paint
// call and `present()` is not observable from application code. Stage
// definitions per the shared contract: input_received, edit_applied,
// layout_begin, layout_end, render_begin, render_end, frame_submit.
use gpui::EntityInputHandler;

/// Persistent per-frame hook: record `frame_submit` and re-register.
/// Disabled for Phase A0: on_next_frame never fired on Windows 0.2.2 (see
/// note above); kept here to re-enable once the platform timing is understood.
#[allow(dead_code)]
fn frame_submit_hook(window: &mut Window, _cx: &mut App) {
    instrument::record(Stage::FrameSubmit);
    window.on_next_frame(frame_submit_hook);
}

fn log_window_state(label: &str, window: &mut Window, _cx: &mut App) {
    let bounds = window.bounds();
    let scale = window.scale_factor();
    let rem = window.rem_size();
    println!(
        "[window] {label}: {}x{} @({},{}), scale_factor={scale}, rem_size={}px",
        f32::from(bounds.size.width),
        f32::from(bounds.size.height),
        f32::from(bounds.origin.x),
        f32::from(bounds.origin.y),
        f32::from(rem)
    );
}

fn print_state(view: &Entity<ThinEditor>, cx: &mut App, label: &str) {
    let e = view.read(cx);
    let head: String = e.text.chars().take(64).collect();
    println!(
        "[smoke] {label}: lines={} cursor={} sel={:?} marked={:?} scroll_y={} text_head={:?} line_starts={:?}",
        e.line_count(),
        e.cursor_offset(),
        e.selection,
        e.marked_range,
        f32::from(e.scroll_y),
        head,
        &e.line_starts[..e.line_starts.len().min(3)]
    );
}

/// Deterministic self-test: one step per frame, driven through the same
/// input-handler + keystroke-dispatch paths the platform uses.
fn smoke_step(view: Entity<ThinEditor>, step: usize) -> impl FnOnce(&mut Window, &mut App) {
    move |window, cx| {
        // Typing workload: steps 1..=100 type one 'a' per frame through
        // the real keystroke dispatch (shared by both MVPs' first-round
        // benchmark — see bench/run-bench.cmd).
        let done = if step >= 1 && step <= 100 {
            window.dispatch_keystroke(Keystroke::parse("a").unwrap(), cx);
            if step == 100 {
                print_state(&view, cx, "after typing x100");
            }
            false
        } else {
            match step {
            0 => {
                window.focus(&view.read(cx).focus_handle);
                print_state(&view, cx, "start");
                false
            }
            101 => {
                // Plain character input (what WM_CHAR / key_char dispatch to).
                view.update(cx, |e, cx| e.replace_text_in_range(None, "Hi你好!", window, cx));
                false
            }
            102 => {
                // IME composition start: mark text without committing.
                view.update(cx, |e, cx| {
                    e.replace_and_mark_text_in_range(None, "世", Some(0..0), window, cx)
                });
                false
            }
            103 => {
                // IME commit: replace the marked range.
                view.update(cx, |e, cx| e.replace_text_in_range(None, "世界", window, cx));
                print_state(&view, cx, "after ime-commit");
                false
            }
            104 => {
                let keymap = cx.key_bindings();
                let keymap = keymap.borrow();
                let matching: Vec<String> = keymap
                    .bindings()
                    .filter(|b| b.keystrokes().iter().any(|k| k.key() == "end"))
                    .map(|b| format!("{} -> {}", b.keystrokes()[0], b.action().name()))
                    .collect();
                println!(
                    "[smoke] keymap bindings={} end-bindings={matching:?}",
                    keymap.bindings().len()
                );
                drop(keymap);
                window.dispatch_keystroke(Keystroke::parse("end").unwrap(), cx);
                print_state(&view, cx, "after keystroke end");
                false
            }
            105 => {
                window.dispatch_keystroke(Keystroke::parse("backspace").unwrap(), cx);
                print_state(&view, cx, "after backspace");
                false
            }
            106 => {
                window.dispatch_keystroke(Keystroke::parse("backspace").unwrap(), cx);
                print_state(&view, cx, "after backspace2");
                false
            }
            107 => {
                window.dispatch_keystroke(Keystroke::parse("enter").unwrap(), cx);
                print_state(&view, cx, "after enter");
                false
            }
            108 => {
                window.dispatch_keystroke(Keystroke::parse("cmd-a").unwrap(), cx);
                print_state(&view, cx, "after cmd-a");
                false
            }
            109 => {
                // Scroll by two lines, as the wheel handler would.
                view.update(cx, |e, cx| {
                    e.scroll_y = px(f32::from(e.scroll_y) + 2.0 * f32::from(e.line_height));
                    cx.notify();
                });
                false
            }
            110 => {
                window.resize(size(px(1200.0), px(800.0)));
                false
            }
            111 => {
                print_state(&view, cx, "final");
                instrument::dump("smoke", 1200);
                cx.quit();
                true
            }
            _ => true,
            }
        };
        if !done {
            window.defer(cx, smoke_step(view, step + 1));
        }
    }
}

fn main() {
    let smoke = std::env::args().any(|a| a == "--smoke");
    // --file <path>: load a corpus document instead of the built-in seed
    // (parity with mvp/pocketjs's --file; both MVPs then edit the same
    // bytes).
    let seed: String = std::env::args()
        .collect::<Vec<_>>()
        .windows(2)
        .find(|w| w[0] == "--file")
        .and_then(|w| std::fs::read_to_string(&w[1]).ok())
        .unwrap_or_else(|| editor::ThinEditor::DEFAULT_SEED.to_string());
    // Instrumentation skeleton: all seven stages are observable on GPUI.
            instrument::init(8192, vec!["frame_submit"]);

    Application::new().run(move |cx: &mut App| {
        // Key bindings must be registered before opening the window: the
        // window snapshots the keymap at creation (matches the official
        // input.rs example).
        cx.bind_keys([
            KeyBinding::new("backspace", editor::Backspace, None),
            KeyBinding::new("delete", editor::Delete, None),
            KeyBinding::new("enter", editor::Enter, None),
            KeyBinding::new("left", editor::Left, None),
            KeyBinding::new("right", editor::Right, None),
            KeyBinding::new("up", editor::Up, None),
            KeyBinding::new("down", editor::Down, None),
            KeyBinding::new("shift-left", editor::SelectLeft, None),
            KeyBinding::new("shift-right", editor::SelectRight, None),
            KeyBinding::new("shift-up", editor::SelectUp, None),
            KeyBinding::new("shift-down", editor::SelectDown, None),
            KeyBinding::new("cmd-a", editor::SelectAll, None),
            KeyBinding::new("home", editor::Home, None),
            KeyBinding::new("end", editor::End, None),
            KeyBinding::new("cmd-c", editor::Copy, None),
            KeyBinding::new("cmd-x", editor::Cut, None),
            KeyBinding::new("cmd-v", editor::Paste, None),
            KeyBinding::new("f1", editor::DumpTrace, None),
            KeyBinding::new("cmd-q", editor::Quit, None),
        ]);

        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(1000.0), px(700.0)),
                        cx,
                    ))),
                    focus: true,
                    show: true,
                    ..Default::default()
                },
                |_, cx| cx.new(|cx| ThinEditor::with_seed(cx, &seed)),
            )
            .unwrap();

        window
            .update(cx, |_, window, cx| {
                window.set_window_title("Markit GPUI Thin Editor (Phase A0)");
                log_window_state("created", window, cx);
                // Initial focus, deferred until after this update completes
                // (window.focus() during the update can schedule a draw that
                // re-enters paint while the root view is still borrowed).
                // Note: focus is only accepted while the window is active
                // (gpui's focus_enabled), so a user click is the reliable path.
                let view = cx.entity();
                window.defer(cx, move |window, cx| {
                    window.focus(&view.read(cx).focus_handle);
                });
                // Resize + HiDPI baseline: log bounds and scale factor on every
                // window bounds change (drag-resize, display scale change).
                cx.observe_window_bounds(window, |_editor, window, cx| {
                    log_window_state("resized", window, cx);
                })
                .detach();
                if smoke {
                    let view = cx.entity();
                    window.defer(cx, smoke_step(view, 0));
                }
                cx.activate(true);
            })
            .unwrap();

        cx.on_action(|_: &editor::Quit, cx| {
            instrument::dump("quit", 500);
            cx.quit();
        });
    });
}
