//! Markit Phase A1 — PocketJS thin editor MVP host.
//!
//! Markit-owned desktop host for the PocketJS thin-editor guest
//! (mvp/pocketjs/app): boot the guest bundle over the flat widget shell,
//! bridge real keyboard/mouse/scroll/resize through the svc channel,
//! render the guest's DrawList with demand rendering, and record the
//! shared 7-stage trace (instrument.rs).
//!
//! Window/typography contract mirrors the GPUI Phase A0 prototype
//! (mvp/gpui): 1000x700 logical, Consolas 18 px, 28 px line height,
//! opaque window, resizable, not always-on-top.
//!
//!   bun tools/build.ts markit-editor ...   (guest bundle → dist/)
//!   cargo run --release                    (interactive)
//!   cargo run --release -- --smoke          (deterministic self-test)
//!   cargo run --release -- --frames 60 --screenshot out.png   (headless)

mod instrument;

use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result, anyhow};
use glam::Vec2;
use pocket3d::gpu::{Gpu, OFFSCREEN_FORMAT, OffscreenTarget};
use pocket3d::input::{EditKey, Input};
use pocket_mod::Guest;
use pocket_ui_wgpu::{UiRenderer, UiSurface};
use pocket_widget::shell::{FlatWidget, WidgetConfig};
use winit::keyboard::KeyCode;

use instrument::Stage;

/// Default logical window size — mirrors the GPUI MVP (1000x700).
const WINDOW_W: u32 = 1000;
const WINDOW_H: u32 = 700;
/// Guest cadence (fixed-step, like every PocketJS host).
const TICK_HZ: f32 = 60.0;
/// FNV-1a 64 over the DrawList words — the dirty signal (embed.rs trick).
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

struct MarkitGame {
    surface: UiSurface,
    guest: Guest,
    renderer: Option<UiRenderer>,
    /// Caret rect reported by the guest (logical px) — IME docking.
    caret_rect: Option<(f32, f32, f32, f32)>,
    /// Document loaded from --file; None = the guest's built-in seed.
    file: Option<PathBuf>,
    /// Current logical viewport (the core's), tracked against the window.
    logical: (u32, u32),
    /// DrawList words of the latest tick + their hash (the dirty signal).
    words: Vec<u32>,
    hash: u64,
    dirty: bool,
    exit: bool,
    booted: bool,
    /// Last (x, y, primary-down) sent over svc.
    last_mouse: Option<(f32, f32, bool)>,
    /// Window scale factor from the latest tick (cursor px → logical).
    scale: f64,
    ticks: u64,
    /// Headless scripting (--type/--key/--scroll events by frame).
    script: Vec<(u64, ScriptEvent)>,
    /// Scripted click: CIRCLE held until this tick.
    script_click_until: u64,
    /// Last guest state echo (smoke prints it on change).
    last_state: Option<String>,
    quit_after: Option<u64>,
    /// --smoke: deterministic self-test driver.
    smoke: bool,
    /// --screenshot: headless run — also echoes guest state (deterministic
    /// verification without a display).
    echo_state: bool,
    // ---- Phase A2 instrumentation (Markit-owned) -------------------------
    /// --perf or PJS_PERF=1 enables per-tick JSONL + the guest perfreq
    /// round-trip (a flag, because WSL-launched Windows processes do not
    /// inherit WSL env vars).
    perf: bool,
    /// --dump-words: print the DrawList words on every dirty tick
    /// (A4-R1 diagnostic — DrawList equivalence checks).
    dump_words: bool,
    /// Duration of the last wgpu render pass (render(), us).
    render_us: u64,
    /// Guest perf reply received this tick (printed by the caller).
    perf_reply: Option<String>,
    // ---- Phase A3-M startup marker (Markit-owned) ------------------------
    /// Process start instant (main entry) — marker deltas are process-
    /// internal, so the external runner needs no clock sync.
    process_t0: Instant,
    /// MARKIT_FIRST_USABLE_FRAME printed once after the first submit.
    first_frame_marked: bool,
}

enum ScriptEvent {
    Click(f32, f32),
    Type(String),
    Key(String),
    Paste(String),
    Scroll(f32),
    Resize(u32, u32),
}

impl MarkitGame {
    fn svc(&self, value: serde_json::Value) {
        self.surface.svc_push(value.to_string());
    }

    /// The svc hello: viewport first, then the document (order matters —
    /// the app lays text out against the viewport it was just told about).
    /// No document = the guest renders its built-in seed (GPUI parity).
    fn send_hello(&mut self) {
        self.svc(serde_json::json!({"t": "hello", "w": self.logical.0, "h": self.logical.1}));
        if let Some(path) = &self.file {
            let text = std::fs::read_to_string(path).unwrap_or_default();
            if !text.is_empty() {
                self.svc(serde_json::json!({"t": "load", "text": text}));
            }
            log::info!("markit: loaded {} ({} bytes)", path.display(), text.len());
        }
    }

    /// Forward this tick's keyboard edits to the guest; returns the number
    /// of edit svc lines pushed (Phase A2 counter).
    fn forward_edits(&mut self, input: &Input) -> usize {
        // Batch runs of typed chars into one line; Ctrl-chords are
        // shortcuts, not text.
        let shift = input.key_down(KeyCode::ShiftLeft) || input.key_down(KeyCode::ShiftRight);
        let primary =
            input.key_down(KeyCode::ControlLeft) || input.key_down(KeyCode::ControlRight);
        let mut edits = 0usize;
        let mut chars = String::new();
        for key in input.edits() {
            let named = match key {
                EditKey::Char(c) => {
                    if !primary {
                        chars.push(*c);
                    }
                    continue;
                }
                EditKey::Backspace => "Backspace",
                EditKey::Delete => "Delete",
                EditKey::Enter => "Enter",
                EditKey::Tab => "Tab",
                EditKey::Left => "Left",
                EditKey::Right => "Right",
                EditKey::Up => "Up",
                EditKey::Down => "Down",
                EditKey::Home => "Home",
                EditKey::End => "End",
                EditKey::PageUp => "PageUp",
                EditKey::PageDown => "PageDown",
                EditKey::Escape => "Escape",
            };
            if !chars.is_empty() {
                let batch = std::mem::take(&mut chars);
                self.svc(serde_json::json!({"t": "ch", "s": batch}));
                edits += 1;
                instrument::record(Stage::EditApplied);
            }
            self.svc(serde_json::json!({"t": "key", "k": named, "sh": shift}));
            edits += 1;
            instrument::record(Stage::EditApplied);
        }
        if !chars.is_empty() {
            self.svc(serde_json::json!({"t": "ch", "s": chars}));
            edits += 1;
            instrument::record(Stage::EditApplied);
        }

        // Primary-modifier chords (Ctrl on Windows/Linux — GPUI's cmd maps
        // to Ctrl on Windows). Quit lives in the host; editing chords go
        // to the guest as named keys. Clipboard/IME chords are DEFERRED on
        // Phase A1 (protocol reserved).
        if primary {
            if input.key_pressed(KeyCode::KeyQ) {
                self.exit = true;
            }
            if input.key_pressed(KeyCode::KeyA) {
                self.svc(serde_json::json!({"t": "key", "k": "SelectAll", "sh": false}));
                edits += 1;
                instrument::record(Stage::EditApplied);
            }
        }
        edits
    }

    /// Run due scripted events; returns the number of edit svc lines pushed
    /// (Phase A2 counter) and whether any event fired.
    fn run_script(&mut self) -> (usize, bool) {
        let mut edits = 0usize;
        let mut fired = false;
        let due: Vec<usize> = self
            .script
            .iter()
            .enumerate()
            .filter(|(_, (at, _))| *at == self.ticks)
            .map(|(i, _)| i)
            .collect();
        for i in due.into_iter().rev() {
            let (_, ev) = self.script.remove(i);
            fired = true;
            match ev {
                ScriptEvent::Click(x, y) => {
                    // Mirrors the real-mouse path: press then release.
                    self.svc(serde_json::json!({"t": "mouse", "x": x, "y": y, "d": true}));
                    self.svc(serde_json::json!({"t": "mouse", "x": x, "y": y, "d": false}));
                    self.script_click_until = self.ticks + 4;
                    instrument::record(Stage::InputReceived);
                }
                ScriptEvent::Type(s) => {
                    self.svc(serde_json::json!({"t": "ch", "s": s}));
                    edits += 1;
                    instrument::record(Stage::InputReceived);
                    instrument::record(Stage::EditApplied);
                }
                ScriptEvent::Key(k) => {
                    self.svc(serde_json::json!({"t": "key", "k": k}));
                    edits += 1;
                    instrument::record(Stage::InputReceived);
                    instrument::record(Stage::EditApplied);
                }
                ScriptEvent::Paste(text) => {
                    self.svc(serde_json::json!({"t": "paste", "text": text}));
                    edits += 1;
                    instrument::record(Stage::InputReceived);
                    instrument::record(Stage::EditApplied);
                }
                ScriptEvent::Scroll(dy) => {
                    self.svc(serde_json::json!({"t": "scroll", "dy": dy}));
                    instrument::record(Stage::InputReceived);
                }
                ScriptEvent::Resize(w, h) => {
                    self.logical = (w, h);
                    self.surface
                        .with_ui(|ui| ui.set_viewport(w as f32, h as f32));
                    self.svc(serde_json::json!({"t": "resize", "w": w, "h": h}));
                }
            }
        }
        (edits, fired)
    }
}

impl FlatWidget for MarkitGame {
    fn init(&mut self, gpu: &Gpu, format: wgpu::TextureFormat) -> Result<()> {
        self.renderer = Some(UiRenderer::new(gpu, format));
        Ok(())
    }

    fn tick(&mut self, _dt: f32, input: &Input, window_px: (u32, u32), scale: f64) -> Result<()> {
        self.scale = scale;
        if !self.booted {
            self.booted = true;
            self.send_hello();
        }

        // Window → core viewport. Live resizes relayout the core and tell
        // the app (which re-lays out against the new size).
        let logical = (
            ((window_px.0 as f64 / scale).round() as u32).max(1),
            ((window_px.1 as f64 / scale).round() as u32).max(1),
        );
        if logical != self.logical {
            self.logical = logical;
            self.surface
                .with_ui(|ui| ui.set_viewport(logical.0 as f32, logical.1 as f32));
            self.svc(serde_json::json!({"t": "resize", "w": logical.0, "h": logical.1}));
        }

        // Keyboard / wheel / pointer → svc lines (logical px).
        let had_input = input.edits().len() > 0
            || input.ime_events().len() > 0
            || input.scroll().y != 0.0
            || input.cursor().is_some()
            || input.mouse_button_pressed(winit::event::MouseButton::Left)
            || input.mouse_button_down(winit::event::MouseButton::Left);
        if had_input {
            instrument::record(Stage::InputReceived);
        }

        let mut edits_pushed = self.forward_edits(input);
        let scroll = input.scroll();
        if scroll.y != 0.0 {
            self.svc(serde_json::json!({"t": "scroll", "dy": scroll.y / scale as f32}));
        }
        let mut script_fired = false;
        if !self.script.is_empty() {
            let (n, f) = self.run_script();
            edits_pushed += n;
            script_fired = f;
        }
        let had_input = had_input || script_fired;
        let script_down = self.ticks < self.script_click_until;
        let pressed_edge = input.mouse_button_pressed(winit::event::MouseButton::Left);
        let level_down = input.mouse_button_down(winit::event::MouseButton::Left) || script_down;
        let mouse_down = level_down || pressed_edge;
        let pos = input.cursor().map(|c| (c.x / scale as f32, c.y / scale as f32));
        let shift = input.key_down(KeyCode::ShiftLeft) || input.key_down(KeyCode::ShiftRight);
        if let Some((x, y)) = pos {
            if pressed_edge && !level_down {
                self.svc(serde_json::json!({"t": "mouse", "x": x, "y": y, "d": true, "sh": shift}));
                self.svc(serde_json::json!({"t": "mouse", "x": x, "y": y, "d": false, "sh": shift}));
                self.last_mouse = Some((x, y, false));
            } else {
                let m = (x, y, mouse_down);
                if self.last_mouse != Some(m) {
                    self.last_mouse = Some(m);
                    self.svc(
                        serde_json::json!({"t": "mouse", "x": x, "y": y, "d": mouse_down, "sh": shift}),
                    );
                }
            }
        }

        // Phase A2: ask the guest for its counter dump two ticks before an
        // auto-quit so the reply lands inside a normal tick (bench mode).
        if self.perf
            && let Some(q) = self.quit_after
            && self.ticks + 2 == q
        {
            self.svc(serde_json::json!({"t": "perfreq"}));
        }

        // The guest turn (exactly one per tick). Layout = DrawList
        // regeneration inside surface.tick.
        instrument::record(Stage::LayoutBegin);
        let gf_t0 = Instant::now();
        self.guest.frame(0)?;
        let gf_us = gf_t0.elapsed().as_micros() as u64;
        let ct_t0 = Instant::now();
        self.surface.tick();
        let ct_us = ct_t0.elapsed().as_micros() as u64;
        instrument::record(Stage::LayoutEnd);

        // Guest → host intents.
        let drained: Vec<String> = self.surface.svc_drain();
        for line in drained {
            match serde_json::from_str::<serde_json::Value>(&line) {
                Ok(v) => match v["t"].as_str() {
                    Some("quit") => self.exit = true,
                    Some("perf") => {
                        if self.perf {
                            self.perf_reply = Some(line);
                        }
                    }
                    Some("caret") => {
                        self.caret_rect = Some((
                            v["x"].as_f64().unwrap_or(0.0) as f32,
                            v["y"].as_f64().unwrap_or(0.0) as f32,
                            1.0,
                            v["h"].as_f64().unwrap_or(28.0) as f32,
                        ));
                    }
                    Some("state") => {
                        let caret = v["caret"].as_u64().unwrap_or(0);
                        let anchor = v["anchor"].as_u64().unwrap_or(0);
                        let scroll_y = v["scrollY"].as_f64().unwrap_or(0.0);
                        let w = v["w"].as_u64().unwrap_or(0);
                        let h = v["h"].as_u64().unwrap_or(0);
                        let head = v["docHead"].as_str().unwrap_or("");
                        let s = format!(
                            "caret={caret} anchor={anchor} scroll_y={scroll_y} vp={w}x{h} text_head={head:?}"
                        );
                        if self.last_state.as_deref() != Some(&s) {
                            if self.smoke || self.echo_state {
                                println!("[state] tick={} {s}", self.ticks);
                            }
                            self.last_state = Some(s);
                        }
                    }
                    other => log::warn!("markit: unknown intent {other:?}"),
                },
                Err(e) => log::warn!("markit: bad svc line from guest: {e}"),
            }
        }

        if let Some(limit) = self.quit_after
            && self.ticks >= limit
        {
            self.exit = true;
        }

        // DrawList content hash → demand rendering.
        let dl_t0 = Instant::now();
        let mut words_len = 0usize;
        let (hash, words) = self.surface.with_ui(|ui| {
            let words = &ui.draw().words;
            words_len = words.len();
            let hash = fnv1a64(words);
            (hash, (hash != self.hash).then(|| words.clone()))
        });
        let dl_us = dl_t0.elapsed().as_micros() as u64;
        if let Some(words) = words {
            if self.dump_words {
                let joined: Vec<String> = words.iter().map(|w| w.to_string()).collect();
                println!("[words] tick={} count={} {}", self.ticks, words.len(), joined.join(","));
            }
            log::debug!("markit: DrawList changed at tick {}", self.ticks);
            self.words = words;
            self.hash = hash;
            self.dirty = true;
        }

        if self.perf {
            // Phase A2: one JSON line per tick (see bench/parse-a2.py).
            println!(
                "{{\"perf\":1,\"tick\":{},\"ev\":{},\"in\":{},\"gf_us\":{},\"ct_us\":{},\"dl_us\":{},\"words\":{},\"dirty\":{},\"r_us\":{}}}",
                self.ticks, edits_pushed, had_input as u8, gf_us, ct_us, dl_us, words_len,
                self.dirty as u8, self.render_us,
            );
        }
        if let Some(reply) = self.perf_reply.take() {
            println!("[perf] {reply}");
        }

        self.ticks += 1;
        Ok(())
    }

    fn take_dirty(&mut self) -> bool {
        std::mem::take(&mut self.dirty)
    }

    fn render(&mut self, gpu: &Gpu, view: &wgpu::TextureView, window_px: (u32, u32)) -> Result<()> {
        let renderer = self.renderer.as_mut().expect("init ran");
        instrument::record(Stage::RenderBegin);
        let r_t0 = Instant::now();
        let scale = if self.scale > 0.0 { self.scale as f32 } else { 1.0 };
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        self.surface.with_ui(|ui| {
            renderer.render_words_scaled(
                gpu,
                ui,
                &self.words,
                &mut encoder,
                view,
                window_px,
                scale,
                // Opaque white surface (a normal desktop editor — not a
                // transparent widget; Phase A1 DEFERS transparency).
                wgpu::LoadOp::Clear(wgpu::Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }),
            )
        })?;
        gpu.queue.submit([encoder.finish()]);
        self.render_us = r_t0.elapsed().as_micros() as u64;
        // Phase A3-M: first usable frame = application-level frame-ready
        // (document visible, editor ready to accept input, first command
        // buffer submitted). No OS present timestamp is available on this
        // host; the marker is labeled as frame-ready in the A3 report.
        if !self.first_frame_marked {
            self.first_frame_marked = true;
            println!("MARKIT_FIRST_USABLE_FRAME {}", self.process_t0.elapsed().as_millis());
        }
        instrument::record(Stage::RenderEnd);
        instrument::record(Stage::FrameSubmit);
        Ok(())
    }

    fn drag_at(&mut self, _cursor: Vec2) -> bool {
        false
    }

    fn resize_at(&mut self, _cursor: Vec2) -> bool {
        false
    }

    fn ime_cursor_area(&mut self) -> Option<(f32, f32, f32, f32)> {
        let s = self.scale as f32;
        self.caret_rect.map(|(x, y, w, h)| (x * s, y * s, w * s, h * s))
    }

    fn wants_exit(&self) -> bool {
        self.exit
    }
}

/// FNV-1a 64 over the DrawList words.
fn fnv1a64(words: &[u32]) -> u64 {
    let mut h: u64 = FNV_OFFSET;
    for w in words {
        for b in w.to_le_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(FNV_PRIME);
        }
    }
    h
}

// ---------------------------------------------------------------------------
// boot + CLI
// ---------------------------------------------------------------------------

struct Args {
    app: String,
    js: Option<PathBuf>,
    pak: Option<PathBuf>,
    file: Option<PathBuf>,
    size: (u32, u32),
    density: u32,
    screenshot: Option<PathBuf>,
    frames: u64,
    script: Vec<(u64, ScriptEvent)>,
    auto_quit: Option<f32>,
    smoke: bool,
    perf: bool,
    /// A4-R1: print the DrawList words as a JSON array on every dirty tick
    /// (diagnostic — DrawList equivalence checks).
    dump_words: bool,
}

fn parse_args() -> Result<Args> {
    let mut args = Args {
        app: "markit-editor".into(),
        js: None,
        pak: None,
        file: None,
        size: (WINDOW_W, WINDOW_H),
        density: 1,
        screenshot: None,
        frames: 60,
        script: Vec::new(),
        auto_quit: None,
        smoke: false,
        perf: std::env::var("PJS_PERF").is_ok(),
        dump_words: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        let mut val = |name: &str| -> Result<String> {
            it.next().ok_or_else(|| anyhow!("{name} needs a value"))
        };
        /// `spec@frame` → (frame, spec).
        fn at(v: &str, flag: &str) -> Result<(u64, String)> {
            let (spec, frame) = v
                .rsplit_once('@')
                .ok_or_else(|| anyhow!("{flag} wants value@frame"))?;
            Ok((frame.parse()?, spec.to_string()))
        }
        match a.as_str() {
            "--app" => args.app = val("--app")?,
            "--js" => args.js = Some(PathBuf::from(val("--js")?)),
            "--pak" => args.pak = Some(PathBuf::from(val("--pak")?)),
            "--file" => args.file = Some(PathBuf::from(val("--file")?)),
            "--width" => args.size.0 = val("--width")?.parse()?,
            "--height" => args.size.1 = val("--height")?.parse()?,
            "--density" => args.density = val("--density")?.parse()?,
            "--screenshot" => args.screenshot = Some(PathBuf::from(val("--screenshot")?)),
            "--frames" => args.frames = val("--frames")?.parse()?,
            "--smoke" => args.smoke = true,
            "--perf" => args.perf = true,
            "--dump-words" => args.dump_words = true,
            "--type" => {
                let (frame, s) = at(&val("--type")?, "--type")?;
                args.script.push((frame, ScriptEvent::Type(s)));
            }
            "--key" => {
                let (frame, k) = at(&val("--key")?, "--key")?;
                args.script.push((frame, ScriptEvent::Key(k)));
            }
            "--paste" => {
                let (frame, text) = at(&val("--paste")?, "--paste")?;
                args.script.push((frame, ScriptEvent::Paste(text)));
            }
            "--click" => {
                let (frame, spec) = at(&val("--click")?, "--click")?;
                let (x, y) = spec
                    .split_once(',')
                    .ok_or_else(|| anyhow!("--click wants x,y@frame"))?;
                args.script
                    .push((frame, ScriptEvent::Click(x.trim().parse()?, y.trim().parse()?)));
            }
            "--scroll" => {
                let (frame, dy) = at(&val("--scroll")?, "--scroll")?;
                args.script.push((frame, ScriptEvent::Scroll(dy.parse()?)));
            }
            "--resize" => {
                let (frame, spec) = at(&val("--resize")?, "--resize")?;
                let (w, h) = spec
                    .split_once(',')
                    .ok_or_else(|| anyhow!("--resize wants w,h@frame"))?;
                args.script.push((
                    frame,
                    ScriptEvent::Resize(w.trim().parse()?, h.trim().parse()?),
                ));
            }
            "--auto-quit" => args.auto_quit = Some(val("--auto-quit")?.parse()?),
            other => return Err(anyhow!("unknown flag {other}")),
        }
    }
    Ok(args)
}

/// `mvp/pocketjs/dist` — relative to this crate in the source tree, or
/// POCKETJS_DIST, or ./dist for standalone binaries.
fn dist_dir() -> Option<PathBuf> {
    if let Ok(d) = std::env::var("POCKETJS_DIST") {
        return Some(PathBuf::from(d));
    }
    let from_manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("dist");
    from_manifest
        .canonicalize()
        .ok()
        .or_else(|| {
            let cwd = PathBuf::from("dist");
            cwd.is_dir().then_some(cwd)
        })
        .or_else(|| {
            // Dev fallback: the vendored repo's own dist.
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../vendor/pocketjs/dist")
                .canonicalize()
                .ok()
        })
}

fn resolve_asset(explicit: Option<PathBuf>, app: &str, ext: &str) -> Result<PathBuf> {
    if let Some(p) = explicit {
        return p
            .canonicalize()
            .with_context(|| format!("missing {}", p.display()));
    }
    let dist =
        dist_dir().ok_or_else(|| anyhow!("cannot find PocketJS dist/ (set POCKETJS_DIST)"))?;
    let candidates = [format!("{app}.{ext}"), format!("{app}-main.{ext}")];
    for c in &candidates {
        let p = dist.join(c);
        if p.is_file() {
            return Ok(p);
        }
    }
    Err(anyhow!(
        "no {ext} for app '{app}' in {} — build it first: bun tools/build.ts {app}",
        dist.display()
    ))
}

/// Boot the guest: feed the pak, mount `ui` (svc included), eval the bundle.
fn boot(args: &Args) -> Result<(Guest, UiSurface)> {
    let js_path = resolve_asset(args.js.clone(), &args.app, "js")?;
    let pak_path = resolve_asset(args.pak.clone(), &args.app, "pak")?;
    let bundle = std::fs::read_to_string(&js_path)
        .with_context(|| format!("reading {}", js_path.display()))?;
    let pak = std::fs::read(&pak_path).with_context(|| format!("reading {}", pak_path.display()))?;

    let surface = UiSurface::new_with_density(
        (args.size.0 as f32, args.size.1 as f32),
        args.density,
    );
    // Identity follows the vendored baseline (note-widget on main uses
    // macos-widget/3; a Windows identity is a PocketJS change — record the
    // failure first if boot ever rejects it).
    surface.set_identity("macos-widget", 3);
    surface.feed_pak(&pak);
    let guest = Guest::new()?;
    surface.mount(&guest)?;
    guest.eval(&args.app, &bundle)?;
    if !guest.has_frame() {
        return Err(anyhow!(
            "bundle evaluated but installed no frame() — is this a PocketJS app?"
        ));
    }
    Ok((guest, surface))
}

/// Deterministic smoke driver — mirrors the GPUI --smoke step order
/// (type, backspace x2, enter, select-all, scroll, resize, dump), minus
/// the IME steps (DEFERRED on Phase A1). Scripted events run on fixed
/// ticks; the guest's state echo is printed as it changes.
fn smoke_script() -> Vec<(u64, ScriptEvent)> {
    vec![
        (2, ScriptEvent::Type("Hi!".into())),
        (4, ScriptEvent::Key("Backspace".into())),
        (6, ScriptEvent::Key("Backspace".into())),
        (8, ScriptEvent::Key("Enter".into())),
        (10, ScriptEvent::Key("SelectAll".into())),
        (12, ScriptEvent::Scroll(56.0)),
        (14, ScriptEvent::Resize(1200, 800)),
    ]
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let mut args = parse_args()?;
    let (guest, surface) = boot(&args)?;

    if args.smoke {
        args.script = smoke_script();
        args.auto_quit = Some(1.0);
    }

    let game = MarkitGame {
        surface,
        guest,
        renderer: None,
        caret_rect: None,
        file: args.file.clone(),
        logical: args.size,
        words: Vec::new(),
        hash: 0,
        dirty: true,
        exit: false,
        booted: false,
        last_mouse: None,
        scale: 1.0,
        ticks: 0,
        script: std::mem::take(&mut args.script),
        script_click_until: 0,
        last_state: None,
        quit_after: args.auto_quit.map(|s| (s * TICK_HZ as f32) as u64),
        smoke: args.smoke,
        echo_state: args.screenshot.is_some(),
        perf: args.perf || std::env::var("PJS_PERF").is_ok(),
        dump_words: args.dump_words,
        render_us: 0,
        perf_reply: None,
        process_t0: Instant::now(),
        first_frame_marked: false,
    };

    if let Some(out) = args.screenshot.clone() {
        headless(game, args, &out)?
    } else {
        // frame_submit is observable in the windowed path (we submit the
        // command buffer ourselves); headless runs mark it unavailable.
        instrument::init(8192, Vec::new());
        if args.smoke {
            println!("[smoke] boot: seed doc, 1000x700, Consolas 18px, line_h=28");
        }
        pocket_widget::run_flat(
            WidgetConfig {
                title: "Markit PocketJS Thin Editor (Phase A1)".into(),
                size: args.size,
                // A normal opaque desktop editor — NOT a transparent
                // widget. (Windows alpha-mode degradation is a known
                // PocketJS gap; Markit does not need transparency.)
                transparent: false,
                resizable: true,
                min_size: (240, 180),
                ime: true,
                ..Default::default()
            },
            game,
        )?;
        instrument::dump("quit", 500);
    }
    Ok(())
}

/// Headless: N fixed ticks at 1x scale (logical == physical), scripted svc
/// events, then one PNG at density scale. No window required; frame_submit
/// is unavailable (nothing presents).
fn headless(mut game: MarkitGame, args: Args, out: &std::path::Path) -> Result<()> {
    instrument::init(8192, vec!["frame_submit"]);
    let gpu = Gpu::new_headless()?;
    game.init(&gpu, OFFSCREEN_FORMAT)?;
    let mut input = Input::default();
    for _ in 0..args.frames {
        // Phase A2: ask the guest for its counter dump on the last frame.
        if game.perf && game.ticks + 1 == args.frames {
            game.svc(serde_json::json!({"t": "perfreq"}));
        }
        // Headless: the "window" tracks the game's logical viewport, so
        // scripted --resize events behave like a real window resize.
        let px = (game.logical.0, game.logical.1);
        game.tick(1.0 / 60.0, &input, px, 1.0)?;
        input.end_frame();
    }
    let scale = args.density.max(1);
    let (w, h) = (args.size.0 * scale, args.size.1 * scale);
    let target = OffscreenTarget::new(&gpu, w, h);
    game.take_dirty();
    let renderer = game.renderer.as_mut().expect("init ran");
    instrument::record(Stage::RenderBegin);
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    game.surface.with_ui(|ui| {
        renderer.render_words_scaled(
            &gpu,
            ui,
            &game.words,
            &mut encoder,
            &target.view,
            (w, h),
            scale as f32,
            wgpu::LoadOp::Clear(wgpu::Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }),
        )
    })?;
    gpu.queue.submit([encoder.finish()]);
    instrument::record(Stage::RenderEnd);
    target.save_png(&gpu, out)?;
    println!(
        "markit: wrote {} after {} frames ({}x{} @{}x)",
        out.display(),
        args.frames,
        w,
        h,
        scale
    );
    instrument::dump("headless", 500);
    Ok(())
}
