//! Markit PocketJS MVP — shared 7-stage trace contract.
//!
//! Same stages, names and dump format as the GPUI MVP
//! (mvp/gpui/src/instrument.rs) so the two implementations can be compared
//! line by line:
//!
//!   input_received  - platform input reached the application
//!   edit_applied    - document mutation applied
//!   layout_begin    - layout phase started
//!   layout_end      - layout phase finished
//!   render_begin    - render/prepaint phase started
//!   render_end      - render phase finished
//!   frame_submit    - frame handed to the platform for presentation
//!
//! Stage semantics on this host (proxies are explicit, never faked):
//!   input_received - the tick consumed at least one host input event
//!   edit_applied   - PROXY: the host forwarded an edit to the guest over
//!                    svc (the guest applies it within the same tick; the
//!                    mutation itself is not observable from the host)
//!   layout_begin   - the guest turn started (guest.frame + surface.tick,
//!                    which regenerates the DrawList)
//!   layout_end     - the guest turn finished (DrawList hash sampled)
//!   render_begin   - the wgpu render pass started
//!   render_end     - the render pass finished
//!   frame_submit   - the command buffer was submitted (present is the
//!                    OS's job; headless runs mark it unavailable)

use std::sync::Mutex;
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
