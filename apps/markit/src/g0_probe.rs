//! G0 baseline probe for the pinned GPUI revision.
//!
//! A minimal windowed capability + instrumentation probe (roadmap G0,
//! docs/product/g0-gpui-baseline.md). NOT an editor: it exists to produce
//! Windows runtime evidence for the baseline freeze (frames, idle wakeups,
//! foreground/background/idle execution primitives, timers, cancellation,
//! clipboard, IME pipeline, font fallback, input→frame instrumentation) and
//! then get out of the way. P0-03 replaces it with the first product slice.
//!
//! Modes:
//!   markit --g0-probe            interactive (click to focus; F1 dump,
//!                                F2 clipboard roundtrip, F3 read, Q quit)
//!   markit --g0-probe --smoke    scripted matrix, auto-quit, G0-prefixed trace

use std::{
    ops::Range,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering::Relaxed},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use gpui::{
    actions, canvas, div, prelude::*, px, rgb, size, App, AppContext, Bounds, ClipboardItem,
    Context, ElementInputHandler, Entity, EntityInputHandler, FocusHandle, Focusable, KeyBinding,
    Pixels, Priority, Render, UTF16Selection, Window, WindowBounds, WindowOptions,
};
use gpui_platform::application;

actions!(g0, [Dump, Quit, WriteClipboard, ReadClipboard]);

/// Timestamp taken at process entry, before any GPUI work.
static T_MAIN: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

fn t_main() -> Instant {
    *T_MAIN.get_or_init(Instant::now)
}

/// Cross-thread instrumentation counters. Draw/frame timestamps come from the
/// render thread (main); background probes log from threadpool threads.
#[derive(Default)]
struct Counters {
    draws: AtomicU64,
    frame_requests: AtomicU64,
    key_actions: AtomicU64,
    mouse_events: AtomicU64,
    ime_composition_updates: AtomicU64,
    ime_commits: AtomicU64,
    /// When true, render re-registers an `on_next_frame` callback each time
    /// one fires (self-sustaining), so frame-request delivery can be measured
    /// while the window is otherwise idle.
    frame_probe_armed: AtomicBool,
    /// Input timestamp recorded at action entry; consumed by the next paint.
    pending_input: Mutex<Option<Instant>>,
    /// (last, max, count) input→paint latencies in microseconds.
    input_latency_us: Mutex<(u64, u64, u64)>,
    first_draw_ms: Mutex<Option<f64>>,
    last_status: Mutex<String>,
}

impl Counters {
    fn note_input(&self) {
        *self.pending_input.lock().unwrap() = Some(Instant::now());
        self.key_actions.fetch_add(1, Relaxed);
    }

    fn note_draw(&self) {
        self.draws.fetch_add(1, Relaxed);
        let mut first = self.first_draw_ms.lock().unwrap();
        if first.is_none() {
            *first = Some(t_main().elapsed().as_secs_f64() * 1000.0);
        }
        drop(first);
        if let Some(t_in) = self.pending_input.lock().unwrap().take() {
            let us = t_in.elapsed().as_micros() as u64;
            let mut lat = self.input_latency_us.lock().unwrap();
            lat.0 = us;
            lat.1 = lat.1.max(us);
            lat.2 += 1;
        }
    }

    fn latency_summary(&self) -> String {
        let (last, max, n) = *self.input_latency_us.lock().unwrap();
        format!("last_us={last} max_us={max} n={n}")
    }
}

/// Self-sustaining `on_next_frame` registration: each delivered callback
/// counts one frame request and re-arms itself while `armed` is set.
fn rearm_frame_probe(counters: Arc<Counters>, window: &mut Window) {
    window.on_next_frame(move |window, _| {
        counters.frame_requests.fetch_add(1, Relaxed);
        if counters.frame_probe_armed.load(Relaxed) {
            rearm_frame_probe(counters.clone(), window);
        }
    });
}

struct G0Probe {
    focus_handle: FocusHandle,
    counters: Arc<Counters>,
    /// Editable buffer driven by the IME/input-handler pipeline.
    buffer: String,
    /// UTF-16 range of the active IME composition, if any.
    marked: Option<Range<usize>>,
    caret_utf16: usize,
    last_bounds: Option<Bounds<Pixels>>,
    last_scale_factor: Option<f32>,
}

impl G0Probe {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            counters: Arc::new(Counters::default()),
            buffer: "Markit G0 probe\nLatin: the quick brown fox 0123\n中文渲染测试：汉字与标点\nEmoji fallback 🙂👍🧑‍💻🌍\ntype here (IME ok): ".into(),
            marked: None,
            caret_utf16: 0,
            last_bounds: None,
            last_scale_factor: None,
        }
    }

    fn set_status(&self, status: String) {
        *self.counters.last_status.lock().unwrap() = status;
    }

    fn utf16_to_byte(&self, ix: usize) -> usize {
        let mut u16_count = 0;
        for (byte, ch) in self.buffer.char_indices() {
            if u16_count >= ix {
                return byte;
            }
            u16_count += ch.len_utf16();
        }
        self.buffer.len()
    }

    fn byte_to_utf16(&self, byte_ix: usize) -> usize {
        self.buffer
            .char_indices()
            .take_while(|(byte, _)| *byte < byte_ix)
            .map(|(_, ch)| ch.len_utf16())
            .sum()
    }

    fn dump(&self) {
        let c = &self.counters;
        log::info!(
            "G0 dump draws={} frame_requests={} key_actions={} mouse={} ime_updates={} ime_commits={} first_draw_ms={:.1} input_latency[{}] status={}",
            c.draws.load(Relaxed),
            c.frame_requests.load(Relaxed),
            c.key_actions.load(Relaxed),
            c.mouse_events.load(Relaxed),
            c.ime_composition_updates.load(Relaxed),
            c.ime_commits.load(Relaxed),
            c.first_draw_ms.lock().unwrap().unwrap_or(f64::NAN),
            c.latency_summary(),
            c.last_status.lock().unwrap(),
        );
    }

    fn clipboard_roundtrip(&mut self, cx: &mut Context<Self>) {
        let token = format!("markit-g0-probe-{}", t_main().elapsed().as_micros());
        cx.write_to_clipboard(ClipboardItem::new_string(token.clone()));
        let read_back = cx.read_from_clipboard().and_then(|item| item.text());
        match read_back {
            Some(text) if text == token => {
                self.set_status(format!("clipboard roundtrip OK ({token})"));
                log::info!("G0 clipboard roundtrip=ok token={token}");
            }
            other => {
                self.set_status(format!("clipboard roundtrip MISMATCH ({other:?})"));
                log::info!("G0 clipboard roundtrip=mismatch wrote={token} read={other:?}");
            }
        }
        cx.notify();
    }
}

impl Focusable for G0Probe {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for G0Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Resize / scale-factor evidence, logged once per change.
        let bounds = window.bounds();
        if self.last_bounds != Some(bounds) {
            self.last_bounds = Some(bounds);
            log::info!(
                "G0 bounds w={:.0} h={:.0}",
                bounds.size.width.as_f32(),
                bounds.size.height.as_f32()
            );
        }
        let scale = window.scale_factor();
        if self.last_scale_factor != Some(scale) {
            self.last_scale_factor = Some(scale);
            log::info!("G0 scale_factor={scale}");
        }

        // Arm the self-sustaining frame-request probe (toggled by the smoke
        // matrix; always on in interactive mode).
        if self.counters.frame_probe_armed.load(Relaxed) {
            rearm_frame_probe(self.counters.clone(), window);
        }

        let counters = self.counters.clone();
        let probe = cx.entity();
        let focus_handle = self.focus_handle.clone();
        let status = self.counters.last_status.lock().unwrap().clone();
        let composition_active = self.marked.is_some();

        div()
            .track_focus(&self.focus_handle)
            .size_full()
            .bg(rgb(0x1e1e2e))
            .p_4()
            .flex()
            .flex_col()
            .gap_2()
            .text_color(rgb(0xcdd6f4))
            .text_size(px(16.0))
            .on_action(cx.listener(|this, _: &Quit, _, cx| {
                this.dump();
                cx.quit();
            }))
            .on_action(cx.listener(|this, _: &Dump, _, cx| {
                this.counters.note_input();
                this.dump();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &WriteClipboard, _, cx| {
                this.counters.note_input();
                this.clipboard_roundtrip(cx);
            }))
            .on_action(cx.listener(|this, _: &ReadClipboard, _, cx| {
                this.counters.note_input();
                match cx.read_from_clipboard().and_then(|i| i.text()) {
                    Some(text) => {
                        this.set_status(format!("clipboard read: {text:?}"));
                        log::info!("G0 clipboard read={text:?}");
                    }
                    None => {
                        this.set_status("clipboard read: EMPTY".to_string());
                        log::info!("G0 clipboard read=empty");
                    }
                }
                cx.notify();
            }))
            .on_mouse_down(gpui::MouseButton::Left, {
                let counters = counters.clone();
                move |_, _, _| {
                    counters.mouse_events.fetch_add(1, Relaxed);
                }
            })
            .child(format!("status: {status}"))
            .child(self.buffer.clone())
            .child(if composition_active {
                "[IME composition active]"
            } else {
                "[no composition]"
            })
            .child("F1 dump | F2 clipboard roundtrip | F3 clipboard read | Q quit")
            .child(canvas(
                move |bounds: Bounds<Pixels>, _, _| bounds,
                move |_, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App| {
                    // Paint hook: the true draw moment for this frame.
                    counters.note_draw();
                    // Attach the input handler so the platform IME pipeline
                    // has a target with up-to-date bounds.
                    window.handle_input(
                        &focus_handle,
                        ElementInputHandler::new(bounds, probe.clone()),
                        cx,
                    );
                },
            ))
    }
}

impl EntityInputHandler for G0Probe {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        adjusted: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let start = self.utf16_to_byte(range.start);
        let end = self.utf16_to_byte(range.end).max(start);
        *adjusted = Some(start..end);
        Some(self.buffer[start..end].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.caret_utf16..self.caret_utf16,
            reversed: false,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked.clone()
    }

    fn unmark_text(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.marked.take().is_some() {
            self.counters.ime_commits.fetch_add(1, Relaxed);
            log::info!("G0 ime commit (unmark) tail={:?}", self.buffer_tail());
        }
        cx.notify();
    }

    fn replace_text_in_range(
        &mut self,
        _range: Option<Range<usize>>,
        text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.marked.is_some() {
            self.counters.ime_commits.fetch_add(1, Relaxed);
            log::info!("G0 ime commit text={text:?}");
        } else {
            log::info!("G0 text replace text={text:?}");
        }
        self.marked = None;
        self.buffer.push_str(text);
        self.caret_utf16 = self.byte_to_utf16(self.buffer.len());
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        _range: Option<Range<usize>>,
        new_text: &str,
        new_selected_range: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.counters.ime_composition_updates.fetch_add(1, Relaxed);
        if self.marked.is_none() {
            log::info!("G0 ime composition start");
        }
        // Replace any previous composition span, then store the new one.
        if let Some(m) = self.marked.take() {
            let start = self.utf16_to_byte(m.start);
            let end = self.utf16_to_byte(m.end).max(start);
            self.buffer.replace_range(start..end, "");
        }
        let start_utf16 = self.byte_to_utf16(self.buffer.len());
        self.buffer.push_str(new_text);
        let end_utf16 = self.byte_to_utf16(self.buffer.len());
        self.marked = Some(new_selected_range.unwrap_or(start_utf16..end_utf16));
        log::info!(
            "G0 ime composition update text={new_text:?} marked={:?}",
            self.marked
        );
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        _range_utf16: Range<usize>,
        element_bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        // Candidate-window docking rect: bottom edge of the text block.
        Some(Bounds {
            origin: gpui::Point {
                x: element_bounds.origin.x,
                y: element_bounds.origin.y + element_bounds.size.height - px(24.0),
            },
            size: gpui::Size {
                width: element_bounds.size.width,
                height: px(24.0),
            },
        })
    }

    fn character_index_for_point(
        &mut self,
        _point: gpui::Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        Some(self.caret_utf16)
    }
}

impl G0Probe {
    fn buffer_tail(&self) -> &str {
        self.buffer.lines().last().unwrap_or("")
    }
}

pub fn run(args: &[String]) {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_micros()
        .try_init();
    let smoke = args.iter().any(|a| a == "--smoke");
    // Pin the process-entry timestamp before any GPUI work runs; otherwise
    // its first use would be inside the first paint (note_draw), making
    // first_draw_ms self-referentially ~0.
    t_main();
    log::info!("G0 probe start smoke={smoke} pid={}", std::process::id());

    application().run(move |cx: &mut App| {
        // Bind keys BEFORE opening the window (Phase A0 prototype finding:
        // otherwise the keymap is empty on Windows).
        cx.bind_keys([
            KeyBinding::new("f1", Dump, None),
            KeyBinding::new("f2", WriteClipboard, None),
            KeyBinding::new("f3", ReadClipboard, None),
            KeyBinding::new("q", Quit, None),
        ]);

        let bounds = Bounds::centered(None, size(px(720.0), px(480.0)), cx);
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |_, cx| cx.new(G0Probe::new),
            )
            .expect("G0: open window");
        cx.activate(true);

        let entity = window
            .update(cx, |probe, _, cx| {
                let focus = probe.focus_handle.clone();
                (cx.entity(), focus)
            })
            .expect("G0: entity");
        // Focus for keyboard + IME (a user click is the reliable focus path
        // while the window is inactive — Phase A0 finding).
        window
            .update(cx, |_, window, cx| window.focus(&entity.1, cx))
            .ok();

        if smoke {
            cx.spawn(async move |cx: &mut gpui::AsyncApp| {
                run_smoke_matrix(entity.0, window, cx).await;
            })
            .detach();
        }
    });
}

/// Run the scripted evidence matrix, then quit. Every step logs `G0 …` lines
/// that the G0 report quotes verbatim.
async fn run_smoke_matrix(
    probe: Entity<G0Probe>,
    window: gpui::WindowHandle<G0Probe>,
    cx: &mut gpui::AsyncApp,
) {
    let counters = probe.update(cx, |p, _| p.counters.clone());
    counters.frame_probe_armed.store(false, Relaxed);
    let bg = cx.background_executor().clone();
    let fg = cx.foreground_executor().clone();

    // P0 — boot evidence.
    bg.timer(Duration::from_millis(300)).await;
    log::info!(
        "G0 boot first_draw_ms={:.1}",
        counters.first_draw_ms.lock().unwrap().unwrap_or(f64::NAN)
    );

    // P1a — passive idle (no pending next-frame callback, nothing dirty):
    // draws must collapse to ~zero. Platform wakeups below the API are
    // measured externally via CPU sampling.
    let (d0, r0) = (
        counters.draws.load(Relaxed),
        counters.frame_requests.load(Relaxed),
    );
    bg.timer(Duration::from_millis(2000)).await;
    let (d1, r1) = (
        counters.draws.load(Relaxed),
        counters.frame_requests.load(Relaxed),
    );
    log::info!(
        "G0 idle_passive_2s draws={} frame_requests={} (draws/s={:.1} wakes/s={:.1})",
        d1 - d0,
        r1 - r0,
        (d1 - d0) as f64 / 2.0,
        (r1 - r0) as f64 / 2.0,
    );

    // P1b — sustained on_next_frame registration while otherwise idle:
    // measures how often the platform delivers frame requests to a clean
    // window that has a pending callback.
    counters.frame_probe_armed.store(true, Relaxed);
    probe.update(cx, |_, cx| cx.notify()); // one render to arm the loop
    let (d0, r0) = (
        counters.draws.load(Relaxed),
        counters.frame_requests.load(Relaxed),
    );
    bg.timer(Duration::from_millis(2000)).await;
    let (d1, r1) = (
        counters.draws.load(Relaxed),
        counters.frame_requests.load(Relaxed),
    );
    log::info!(
        "G0 idle_nextframe_2s draws={} frame_requests={} (draws/s={:.1} wakes/s={:.1})",
        d1 - d0,
        r1 - r0,
        (d1 - d0) as f64 / 2.0,
        (r1 - r0) as f64 / 2.0,
    );
    counters.frame_probe_armed.store(false, Relaxed);

    // P2 — demand redraw: notify at ~60 Hz for 1.2 s; draws should track.
    let (d0, r0) = (
        counters.draws.load(Relaxed),
        counters.frame_requests.load(Relaxed),
    );
    let mut notifies = 0u64;
    let t_start = Instant::now();
    while t_start.elapsed() < Duration::from_millis(1200) {
        probe.update(cx, |_, cx| cx.notify());
        notifies += 1;
        bg.timer(Duration::from_millis(16)).await;
    }
    let (d1, r1) = (
        counters.draws.load(Relaxed),
        counters.frame_requests.load(Relaxed),
    );
    log::info!(
        "G0 demand_1p2s notifies={notifies} draws={} frame_requests={}",
        d1 - d0,
        r1 - r0
    );

    // P3 — idle scheduling semantics. Source verdict (Windows): no metered
    // idle; spawn_when_idle is ordinary low-priority main-thread work. Verify
    // behaviorally: idle tasks cannot run while the main thread is blocked.
    let idle_remaining = fg.idle_time_remaining();
    let busy_until: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));
    let idle_ran = Arc::new(AtomicU64::new(0));
    let idle_during_busy = Arc::new(AtomicU64::new(0));
    *busy_until.lock().unwrap() = Some(Instant::now() + Duration::from_millis(200));
    for _ in 0..100 {
        let (ran, during, until) = (
            idle_ran.clone(),
            idle_during_busy.clone(),
            busy_until.clone(),
        );
        fg.spawn_when_idle(None, async move {
            ran.fetch_add(1, Relaxed);
            if let Some(until) = *until.lock().unwrap() {
                if Instant::now() < until {
                    during.fetch_add(1, Relaxed);
                }
            }
        })
        .detach();
    }
    // Occupy the main thread: idle work is main-thread queued on Windows, so
    // none of the 100 tasks may run before the busy window closes.
    std::thread::sleep(Duration::from_millis(200));
    let ran_after_busy = idle_ran.load(Relaxed);
    bg.timer(Duration::from_millis(300)).await;
    log::info!(
        "G0 idle_sched idle_time_remaining={idle_remaining:?} ran_right_after_busy={ran_after_busy} ran_total={} during_busy={} (expect remaining=None during_busy=0)",
        idle_ran.load(Relaxed),
        idle_during_busy.load(Relaxed),
    );

    // P4 — background priority: Windows threadpool TP_CALLBACK_PRIORITY mapping.
    let order: Arc<Mutex<Vec<&'static str>>> = Arc::new(Mutex::new(Vec::new()));
    for _ in 0..12 {
        for (prio, tag) in [
            (Priority::Low, "L"),
            (Priority::Medium, "M"),
            (Priority::High, "H"),
        ] {
            let order = order.clone();
            bg.spawn_with_priority(prio, async move {
                // Tiny fixed workload so threadpool scheduling, not the work,
                // dominates the completion order.
                let mut x = 0u64;
                for i in 0..200_000u64 {
                    x = x.wrapping_add(i);
                }
                std::hint::black_box(x);
                order.lock().unwrap().push(tag);
            })
            .detach();
        }
    }
    bg.timer(Duration::from_millis(600)).await;
    let order = order.lock().unwrap().clone();
    let first12: String = order.iter().take(12).copied().collect();
    let count = |t: &str| order.iter().filter(|x| **x == t).count();
    log::info!(
        "G0 bg_priority first12={first12} total={} L={} M={} H={}",
        order.len(),
        count("L"),
        count("M"),
        count("H")
    );

    // P5 — timer granularity.
    for i in 0..4 {
        let t = Instant::now();
        bg.timer(Duration::from_millis(50)).await;
        log::info!(
            "G0 timer {} requested_ms=50 actual_ms={:.2}",
            i + 1,
            t.elapsed().as_secs_f64() * 1000.0
        );
    }

    // P6 — cancellation by drop (yield-point cancellation, no preemption).
    let ran = Arc::new(AtomicU64::new(0));
    let task = {
        let (ran, bg2) = (ran.clone(), bg.clone());
        bg.spawn(async move {
            bg2.timer(Duration::from_millis(120)).await;
            ran.fetch_add(1, Relaxed);
        })
    };
    drop(task); // dropping the Task cancels the future.
    bg.timer(Duration::from_millis(400)).await;
    log::info!("G0 cancel_by_drop ran={} (expect 0)", ran.load(Relaxed));

    // P7 — clipboard roundtrip (leaves the token in the OS clipboard for
    // external verification by the host runner).
    probe.update(cx, |p, cx| {
        p.counters.note_input();
        p.clipboard_roundtrip(cx);
    });

    // P8 — programmatic resize → relayout/repaint.
    let draws_before = counters.draws.load(Relaxed);
    window
        .update(cx, |_, window, _| window.resize(size(px(900.0), px(600.0))))
        .ok();
    bg.timer(Duration::from_millis(400)).await;
    log::info!(
        "G0 resize 720x480->900x600 draws_after={}",
        counters.draws.load(Relaxed) - draws_before
    );

    log::info!("G0 MATRIX DONE");
    bg.timer(Duration::from_millis(200)).await;
    probe.update(cx, |p, _| p.dump());
    cx.update(|cx| cx.quit());
}
