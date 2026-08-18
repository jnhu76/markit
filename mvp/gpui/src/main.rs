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
mod a2;

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

/// Deterministic A2 driver — one step per frame, like smoke, but scoped to
/// the Phase A2 experiments: place the caret at a scripted position, then
/// type `n` chars (or `n` no-op redraws) through the real input path.
///
///   --a2-mode pos --a2-pos <begin|q1|mid|q3|end> [--a2-n 50]
///   --a2-mode vp  --a2-vp <inside|near|far>      [--a2-n 50]
///   --a2-mode static                             [--a2-n 50]
///   --a2-mode scale                              [--a2-n 100]  (caret at 0,
///      typing only — the A1 workload's typing segment, no IME steps)
#[derive(Clone, Copy, Debug)]
enum A2Mode {
    Pos(f64),
    VpLine(usize),
    VpFar,
    Static,
    Scale,
}

#[derive(Clone, Copy, Debug)]
struct A2Spec {
    mode: A2Mode,
    n: usize,
}

fn parse_a2() -> Option<A2Spec> {
    let args: Vec<String> = std::env::args().collect();
    let mut mode = None;
    let mut n = 50usize;
    let mut it = args.iter().skip(1);
    let mut pending: Option<&str> = None; // --a2-mode value when not yet consumed
    while let Some(a) = it.next() {
        let mut next_val = || it.next().map(|s| s.as_str());
        match a.as_str() {
            "--a2-mode" => {
                let m = next_val()?;
                mode = Some(match m {
                    "pos" => {
                        let mut pos = "begin";
                        while let Some(v) = next_val() {
                            match v {
                                "--a2-pos" => {
                                    pos = next_val()?;
                                    break;
                                }
                                "--a2-n" => n = next_val()?.parse().ok()?,
                                _ => {}
                            }
                        }
                        match pos {
                            "begin" => A2Mode::Pos(0.0),
                            "q1" => A2Mode::Pos(0.25),
                            "mid" => A2Mode::Pos(0.5),
                            "q3" => A2Mode::Pos(0.75),
                            "end" => A2Mode::Pos(1.0),
                            _ => return None,
                        }
                    }
                    "vp" => {
                        let mut vp = "inside";
                        while let Some(v) = next_val() {
                            match v {
                                "--a2-vp" => {
                                    vp = next_val()?;
                                    break;
                                }
                                "--a2-n" => n = next_val()?.parse().ok()?,
                                _ => {}
                            }
                        }
                        match vp {
                            "inside" => A2Mode::VpLine(10),
                            "near" => A2Mode::VpLine(30),
                            "far" => A2Mode::VpFar,
                            _ => return None,
                        }
                    }
                    "static" => A2Mode::Static,
                    "scale" => A2Mode::Scale,
                    _ => return None,
                });
            }
            "--a2-n" => n = next_val()?.parse().ok()?,
            _ => {
                if let Some(m) = pending.take() {
                    let _ = m;
                }
            }
        }
    }
    mode.map(|mode| A2Spec { mode, n })
}

fn a2_step(view: Entity<ThinEditor>, spec: A2Spec, step: usize) -> impl FnOnce(&mut Window, &mut App) {
    move |window, cx| {
        let n = spec.n;
        let done = match step {
            0 => {
                window.focus(&view.read(cx).focus_handle);
                print_state(&view, cx, "a2 start");
                false
            }
            1 => {
                // Place the caret WITHOUT scrolling (no ensure_cursor_visible):
                // for vp experiments the edit point must stay outside the
                // viewport until the first typed char scrolls it in.
                view.update(cx, |e, _| {
                    let len = e.text.len();
                    let offset = match spec.mode {
                        A2Mode::Pos(f) => (len as f64 * f) as usize,
                        A2Mode::VpLine(l) => {
                            let idx = l.min(e.line_count());
                            e.line_starts[idx]
                        }
                        A2Mode::VpFar => len / 2,
                        A2Mode::Static | A2Mode::Scale => 0,
                    };
                    let offset = offset.min(len);
                    e.selection = offset..offset;
                    e.selection_reversed = false;
                    e.preferred_x = None;
                });
                println!(
                    "[a2] mode={:?} n={} caret={}",
                    spec.mode,
                    n,
                    view.read(cx).cursor_offset()
                );
                false
            }
            s if s >= 2 && s < 2 + n => {
                match spec.mode {
                    A2Mode::Static => {
                        // Redraw with no document change (static-frame control).
                        view.update(cx, |_e, cx| cx.notify());
                    }
                    _ => {
                        window.dispatch_keystroke(Keystroke::parse("a").unwrap(), cx);
                    }
                }
                false
            }
            s if s == 2 + n => {
                print_state(&view, cx, "a2 final");
                instrument::dump("a2", 5000);
                cx.quit();
                true
            }
            _ => true,
        };
        if !done {
            window.defer(cx, a2_step(view, spec, step + 1));
        }
    }
}

fn main() {
    // Phase A3-M: anchor the startup marker at the process entry point.
    instrument::mark_process_start();
    let smoke = std::env::args().any(|a| a == "--smoke");
    let a2 = parse_a2();
    // --file <path>: load a corpus document instead of the built-in seed
    // (A1-era parity with the removed PocketJS MVP's --file; both MVPs
    // then edited the same bytes).
    let seed: String = std::env::args()
        .collect::<Vec<_>>()
        .windows(2)
        .find(|w| w[0] == "--file")
        .and_then(|w| std::fs::read_to_string(&w[1]).ok())
        .unwrap_or_else(|| editor::ThinEditor::DEFAULT_SEED.to_string());
    // Instrumentation skeleton: all seven stages are observable on GPUI.
    instrument::init(8192, vec!["frame_submit"]);
    // --a2 (or GPUI_A2=1) enables the Phase A2 JSONL emission; counters
    // themselves are always collected.
    let a2_on = std::env::args().any(|a| a == "--a2") || std::env::var("GPUI_A2").is_ok();
    a2::set_enabled(a2_on);

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
                if let Some(spec) = a2 {
                    let view = cx.entity();
                    window.defer(cx, a2_step(view, spec, 0));
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
