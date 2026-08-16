//! Minimal instrumentation skeleton for Phase A0 (feasibility spike).
//!
//! Per the experiment design (docs/Markit_Phase0_..._v0.1), we only reserve the
//! timestamp contract here — no benchmark system yet. Stages follow the E2E
//! pipeline decomposition (HotMobile 2017): input -> edit -> layout -> render -> submit.
//!
//! Contract (shared with a future mvp-pocketjs):
//!   input_received  - platform input reached the application
//!   edit_applied    - document mutation applied
//!   layout_begin    - layout phase started
//!   layout_end      - layout phase finished
//!   render_begin    - render/prepaint phase started
//!   render_end      - render phase finished
//!   frame_submit    - frame handed to the platform for presentation
//!
//! A stage may be `unavailable` on a framework; each implementation records
//! what it can observe and marks the rest as unavailable.

use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    InputReceived,
    EditApplied,
    LayoutBegin,
    LayoutEnd,
    RenderBegin,
    RenderEnd,
    FrameSubmit,
}

impl Stage {
    pub fn name(self) -> &'static str {
        match self {
            Stage::InputReceived => "input_received",
            Stage::EditApplied => "edit_applied",
            Stage::LayoutBegin => "layout_begin",
            Stage::LayoutEnd => "layout_end",
            Stage::RenderBegin => "render_begin",
            Stage::RenderEnd => "render_end",
            Stage::FrameSubmit => "frame_submit",
        }
    }
}

/// Ring buffer of (stage, ns-since-epoch) samples. Thread-safe; the spike only
/// ever records from the UI thread, so contention is not a concern.
pub struct Trace {
    epoch: Instant,
    events: Vec<(Stage, u128)>,
    cap: usize,
    dropped: u64,
    /// Stages this implementation cannot observe (set at startup).
    pub unavailable: Vec<&'static str>,
}

impl Trace {
    pub fn new(cap: usize) -> Self {
        Self {
            epoch: Instant::now(),
            events: Vec::with_capacity(cap),
            cap,
            dropped: 0,
            unavailable: Vec::new(),
        }
    }

    pub fn record(&mut self, stage: Stage) {
        let now = self.epoch.elapsed().as_nanos();
        if self.events.len() == self.cap {
            self.events.remove(0);
            self.dropped += 1;
        }
        self.events.push((stage, now));
    }

    /// Dump the last `n` events plus per-stage counts to stdout.
    pub fn dump(&self, label: &str, n: usize) {
        let mut counts = [0u64; 7];
        for (stage, _) in &self.events {
            counts[*stage as usize] += 1;
        }
        println!("\n=== Trace dump: {label} ===");
        println!(
            "  events={} dropped={} unavailable={:?}",
            self.events.len(),
            self.dropped,
            self.unavailable
        );
        let names = [
            "input_received",
            "edit_applied",
            "layout_begin",
            "layout_end",
            "render_begin",
            "render_end",
            "frame_submit",
        ];
        for (i, name) in names.iter().enumerate() {
            println!("  {name}: {}", counts[i]);
        }
        let start = self.events.len().saturating_sub(n);
        for (stage, t) in &self.events[start..] {
            println!("  +{t:>12} ns  {}", stage.name());
        }
        println!("=== end dump ===\n");
    }
}

static TRACE: Mutex<Option<Trace>> = Mutex::new(None);

pub fn init(cap: usize, unavailable: Vec<&'static str>) {
    let mut guard = TRACE.lock().unwrap();
    let mut trace = Trace::new(cap);
    trace.unavailable = unavailable;
    *guard = Some(trace);
}

pub fn record(stage: Stage) {
    if let Ok(mut guard) = TRACE.lock() {
        if let Some(trace) = guard.as_mut() {
            trace.record(stage);
        }
    }
}

pub fn dump(label: &str, n: usize) {
    if let Ok(guard) = TRACE.lock() {
        if let Some(trace) = guard.as_ref() {
            trace.dump(label, n);
        }
    }
}

// ---- Phase A3-M: first-usable-frame marker (Markit-owned) -----------------
// Both MVPs print the same line so the external runner can measure
// process startup -> first usable frame. The delta is process-internal
// (Instant since main entry), so the runner needs no clock sync. This is
// application-level frame-ready — no OS present timestamp is available on
// this host (A1-known; labeled as frame-ready in the A3 report).

static PROCESS_START: OnceLock<Instant> = OnceLock::new();

/// Record the process start instant (first line of main()).
pub fn mark_process_start() {
    PROCESS_START.get_or_init(Instant::now);
}

/// Print the marker once, at the first frame whose content is render-ready.
pub fn first_usable_frame() {
    static DONE: AtomicBool = AtomicBool::new(false);
    if DONE.swap(true, Ordering::SeqCst) {
        return;
    }
    if let Some(t0) = PROCESS_START.get() {
        println!("MARKIT_FIRST_USABLE_FRAME {}", t0.elapsed().as_millis());
    }
}
