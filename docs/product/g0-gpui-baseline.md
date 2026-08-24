# Markit — G0 Product GPUI Baseline (frozen)

Status: **frozen dependency decision (2026-08-22)** — roadmap G0.
Re-validated 2026-08-23 after the PR #13 review round: IME input-handler
semantics corrected (UTF-16 boundary contract, marked-range covers the whole
composition, `range=None` commits replace the marked span), the frame counter
renamed to `next_frame_callbacks` (it counts `on_next_frame` deliveries, not
platform frame requests), real Microsoft Pinyin start/update/commit/cancel
evidence added, and HiDPI validated at 125 %. The canonical matrix quoted
below is the 2026-08-23 100 % rerun with the review-fixed probe; curated
artifacts are committed under `results/evidence/g0/`.

This document records the G0 baseline selection: what was compared, what was
pinned, what the pinned revision actually provides on Windows (source +
runtime evidence), and the policy for changing the pin. It exists so P0-03+
never has to re-derive these facts and so a future pin change knows exactly
which regression matrix must rerun.

G0 answers only: *what exact GPUI revision does Markit product development
pin, and what execution/platform primitives does that revision actually
provide on Windows?* It does not decide Markit's Markdown engine, scheduler
architecture, plugin execution, or buffer structure.

## 1. Frozen baseline

```text
repository:  https://github.com/zed-industries/zed
revision:    eb8e1c8b5502b7007465fbbc465f4a736fa39210
release:     Zed v1.16.1 stable (published 2026-08-19)
crates:      gpui (crate version string "0.2.2") + gpui_platform (0.1.0)
             + gpui_windows (0.1.0, pulled via gpui_platform on Windows)
license:     Apache-2.0 (gpui, gpui_platform, gpui_windows crate manifests
             each declare Apache-2.0; the repo root carries dual
             LICENSE-APACHE / LICENSE-GPL but these crates are published and
             declared under Apache-2.0)
```

Product dependency declaration (workspace root `Cargo.toml`):

```toml
gpui = { git = "https://github.com/zed-industries/zed", rev = "eb8e1c8b5502b7007465fbbc465f4a736fa39210", default-features = false }
gpui_platform = { git = "https://github.com/zed-industries/zed", rev = "eb8e1c8b5502b7007465fbbc465f4a736fa39210", default-features = false }
```

Notes on the declaration:

- The pin is an **immutable commit SHA**, never a branch or moving tag.
- The dependency lives in the product workspace and is consumed only by
  GPUI-facing crates (`apps/markit`: the `editor` product feature since
  P0-03, default on; the `g0-probe` diagnostic feature before that).
  `markit-core` stays GPUI-free (P0 acceptance, G2 gate).
- `mvp/gpui` remains the historical Phase A0 prototype with its own crates.io
  lockfile; it is not rewritten and does not follow the product pin.
- Markit carries **zero local GPUI patches**. Preferred state is zero; any
  future patch must record owner + rebase cost in this file.

## 2. Candidates compared

| | A: prototype baseline | B: current Zed stable (selected) |
|---|---|---|
| Source | crates.io `gpui = "0.2.2"` (checksum `979b45c…bee707`) | zed repo rev `eb8e1c8` (Zed v1.16.1) |
| Crate version string | 0.2.2 | 0.2.2 (**same string, different implementation**) |
| Platform code | monolithic inside `gpui::platform::windows` | split crates: `gpui_platform` → `gpui_windows` / `gpui_macos` / `gpui_linux` / `gpui_wgpu` / `gpui_web` |
| App entry | `gpui::Application::new()` | `gpui_platform::application()` |
| Windows backend | DirectX 11 + DirectComposition (Phase A0 probe) | DirectX 11 + DirectComposition (`gpui_windows/src/directx_renderer.rs`, `directx_devices.rs`) |
| Idle/frame wake | prototype finding: `on_next_frame` never fired on Windows; pre-present not observable | vsync thread re-invalidates every window each vsync; `on_next_frame` delivered on every frame request (§4) |
| Redraw throttling | none observed in prototype | inactive window capped ~30 fps; thermal cap ~60 fps (`gpui/src/window.rs` on_request_frame) |
| Priority support | untested | background threadpool priorities; foreground priority explicitly ignored (§3) |
| Reproducibility | crates.io checksum (fine for the prototype) | git rev — auditable source, exact behavior |
| Relation to upstream | snapshot frozen ~2024-era API; cannot receive Windows fixes | maintained stable channel; Windows-specific fixes flow to future pinned revs |

Selection rationale: candidate A is an unmaintained crates.io snapshot of a
pre-platform-split API; candidate B is the same crate family under active
Windows development, pinned to the current stable release tag's commit. "Newer
is newer" was not the argument — the argument is that B is the only candidate
with a real maintenance path and it passed the full Windows capability audit
below. The prototype remains valid historical feasibility evidence.

## 3. Scheduler primitive matrix (source audit at rev eb8e1c8)

GPUI mechanism ≠ Markit scheduling policy. This table records what GPUI
actually provides; Markit's semantic priority layer (if needed) is future
work under P1, not part of G0.

| Desired Markit primitive | GPUI primitive at eb8e1c8 | Source evidence | Windows support | Semantics / caveat | Markit layer still required |
|---|---|---|---|---|---|
| Dirty invalidation | `WindowInvalidator::invalidate_view` / `set_dirty`; `dirty_views` set; became-dirty edge wakes platform | `crates/gpui/src/window.rs:158-241` | yes | view-granular; first-dirty timestamp tracked (`FrameDirtyAccumulator.dirty_at`) | map core change ranges → view invalidations |
| Demand frame wake | `on_request_frame(RequestFrameOptions{force_render, require_presentation})`; draw only when `is_dirty() \|\| force_render` | `gpui/src/window.rs:1685-1800` | yes (see §4 wake caveat) | clean windows skip draw AND present | none for basic demand rendering |
| Foreground continuation | `ForegroundExecutor::spawn` (main thread) | `gpui/src/executor.rs:314` | yes | **priority ignored for foreground tasks** (`spawn_with_priority` no-op — executor.rs:323-331 comment: "Priority is ignored for foreground tasks - they run in order on the main thread") | Markit priority must not assume foreground priority works |
| Main-thread dispatch priority | `dispatch_on_main_thread(runnable, priority)` → `PriorityQueueSender` | `gpui_windows/src/dispatcher.rs:126-152` | yes | dispatch-level priority IS honored (ordered queue), drained with a **10 ms budget per wake** interleaving paint/input first (`gpui_windows/src/platform.rs:1001-1046`) | usable as low-priority main-thread seam |
| Main-thread idle work | `spawn_when_idle(timeout, future)` | `gpui/src/executor.rs:345` | **fallback** | platform without metered idle → ordinary `Priority::Low` main-thread dispatch, `timeout` ignored (`gpui/src/platform.rs:1017-1026` default; WindowsDispatcher does not override) | Markit must supply its own idle budget; cannot use idle metering on Windows |
| Idle budget visibility | `idle_time_remaining() -> Option<Duration>` | `gpui/src/executor.rs:365`, default `None` at `platform.rs:1028` | **no** (`None`) | no platform overrides the default as of this rev | self-metered budget + yield discipline |
| Background work | `BackgroundExecutor::spawn` (Send futures) | `executor.rs:89`; Windows threadpool `TrySubmitThreadpoolCallback` (`dispatcher.rs:105-113`) | yes | OS thread pool | Markit job semantics on top |
| Background priority | `spawn_with_priority` → `TP_CALLBACK_PRIORITY_{HIGH,NORMAL,LOW}`; `Priority::RealtimeAudio` → dedicated thread at `THREAD_PRIORITY_TIME_CRITICAL` | `dispatcher.rs:104-113, 161-175` | yes | real OS scheduling classes | map Markit observability priorities → these |
| Timers / monotonic clock | `BackgroundExecutor::timer(Duration)`; `dispatch_after` → `CreateThreadpoolTimer`; `increase_timer_resolution()` (`timeBeginPeriod(1)` guard) | `executor.rs:162`; `dispatcher.rs:154-158, 176-183` | yes | timer granularity follows system timer resolution unless the guard is held | calibration uses the guard where 1 ms matters |
| Task cancellation / drop | `Task<T>`: **drop cancels the future**; `detach()` keeps it alive; no preemption of a running poll | `crates/scheduler/src/executor.rs:319-360` | yes (cross-platform) | cancellation is cooperative at yield points only | long synchronous stretches must chunk voluntarily |
| Profiling / timing | `measure(label, f)` env-gated (`ZED_MEASUREMENTS`); crate features `profiler`, `input-latency-histogram`, `frame-duration-histogram`; `InputLatencyTracker`/`InputLatencySnapshot`, `FrameDurationTracker` | `gpui_util/src/lib.rs:154`; `gpui/src/window.rs:1164-1313`; `gpui/Cargo.toml` features | yes | opt-in at build time | Markit instrumentation consumes these seams |
| Input→frame observability | `InputLatencyTracker` (feature-gated) + app-side action/paint timestamps | `window.rs:1243-1313` | yes | histogram is a build feature; timestamps are proxies at app level | probe pattern established in `apps/markit/src/g0_probe.rs` |
| Present observability | draw+present wrapped in `measure("frame duration")`; DWM timing info used for vsync interval (`gpui_windows/src/vsync.rs`) | `window.rs:~1790`; `vsync.rs:60-79` | partial | present-completion timestamp not directly exposed to apps; frame duration wraps draw+present — label as proxy | real input→present tails still need external timing on the host |

## 4. Windows frame wake + idle scheduling verdict

Two questions G0 had to answer with evidence, not assumptions:

**Does Windows GPUI have real metered idle scheduling
(`spawn_when_idle`/`idle_time_remaining`)?**

**No.** Source: the `PlatformDispatcher` trait defaults
(`gpui/src/platform.rs:1017-1030`) implement `dispatch_on_main_thread_when_idle`
as `dispatch_on_main_thread(Priority::Low)` and `idle_time_remaining` as
`None`; `WindowsDispatcher` (`gpui_windows/src/dispatcher.rs:101-183`) does not
override either; no platform crate in the workspace overrides them. Runtime
confirmation: `g0_probe --smoke` logs `idle_sched idle_time_remaining=None` and
idle tasks execute as ordinary queued main-thread work (none ran during a
200 ms blocked-main-thread window). **Consequence for P1:** any Markit idle
budget must be self-metered (deadline + yield), and low-priority main-thread
work relies on the 10 ms drain budget in the Windows message loop, not on
platform idle metering.

**Do idle windows stop requesting frames?**

Split answer. Product-level draws are demand-driven: a clean window performs
no draw and no present (`gpui/src/window.rs:1782` — `if invalidator.is_dirty()
|| force_render`), and inactive windows are throttled to ~30 fps when they do
have pending work. Platform-level wakeups are NOT demand-driven on Windows: a
dedicated vsync thread (`gpui_windows/src/platform.rs:311-352`) calls
`RedrawWindow(RDW_INVALIDATE)` on **every window every vsync**, producing
WM_PAINT → `request_frame` invocations that are then skipped as no-ops for
clean windows. Runtime numbers (idle draw rate ≈ 0, armed `on_next_frame`
delivery rate, idle CPU) are recorded in §5. This is an accepted property of
the baseline, not a Markit defect; Markit's INV-04 claim must therefore be
stated as *"the idle editor does no draw/present work"*, not *"the process
receives no wakeups"*. The probe's counter is named `next_frame_callbacks`
precisely because it observes `on_next_frame` deliveries to the application,
not the platform's WM_PAINT/request-frame traffic underneath.

## 5. Windows real-host evidence (G0 smoke matrix)

Machine/host: recorded per run below. Build: **native release build on the
Windows host** (`cargo build --release -p markit --features g0-probe`,
rustc version recorded in `results/summary/g0/`), run windowed on the
interactive desktop session. Cross-building from WSL2 works for
`cargo check`/clippy but release cross-builds are **unsupported under the
current Linux-host cross-build path at this pinned revision**:
`gpui_windows`'s build script compiles its HLSL shaders with `fxc.exe` only
when the build host is Windows (`#[cfg(target_os = "windows")]` in its
`build.rs`), and the `shaders_bytes.rs` include is release-only — so a
Linux-hosted release build fails with a missing `shaders_bytes.rs`. (This is
a property of the current cross path, not an absolute impossibility:
pre-generated shader bytes, a Wine-hosted fxc, or an upstream build.rs
change would alter it.) Debug builds compile shaders at runtime instead and
do cross-build. Full raw logs stay local under `results/raw/g0/`
(gitignored); the committed record is `results/summary/g0/` plus the curated
key artifacts in `results/evidence/g0/` (canonical trace, build receipts,
IME trace, idle sampling, screenshots, host summary).

<!-- G0-RUN 2026-08-22: host HU (Ryzen 7 5800H, Win11 Pro 10.0.26100,
     2560x1440 @ scale 1, AMD Radeon iGPU); first full matrix. -->
<!-- G0-RUN 2026-08-23: same host. Canonical matrix rerun at scale 100% with
     the review-fixed probe (results/evidence/g0/canonical-smoke.log); HiDPI
     validated at 125% (hidpi-125.log + g0-window-125.png); real MS Pinyin
     IME pass at scale 100% (pinyin-ime.log); external idle sampling at
     125% (external-idle.txt). Verbatim summary: results/summary/g0/. -->

| Check | Result | Evidence |
|---|---|---|
| Release build (native Windows host) | PASS | `Finished release [optimized] in 6m02s` (2026-08-22) + 7.3 s incremental rebuild after the review fixes, zero warnings; receipts in `results/evidence/g0/windows-build-receipt.txt`; release cross-build unsupported under the current Linux-host path (§5 preamble) |
| Create/show window | PASS | `G0 bounds w=720 h=480`; window pixels captured (screenshots at 100 % and 125 %) |
| Resize (programmatic) | PASS | `G0 resize 720x480->900x600 draws_after=1` |
| HiDPI scale factor | PASS @ 1.0 and 1.25 | `G0 scale_factor=1` (2026-08-22 + 2026-08-23 canonical) and `G0 scale_factor=1.25` (2026-08-23, Settings 125 %); DPI-aware capture shows the 720×480 logical window at 918×647 physical px with sharp text (`results/evidence/g0/hidpi-125.log`, `g0-window-125.png`) |
| Latin rendering | PASS | screenshot: "Latin: the quick brown fox 0123" |
| Chinese (CJK) rendering | PASS | screenshot: `中文渲染测试：汉字与标点` as real glyphs; DirectWrite picked Microsoft YaHei UI |
| Emoji / fallback rendering | PASS | screenshot: `🙂👍🧑‍💻🌍` in color |
| Keyboard actions (F1/F2/F3/Q) | PASS (F1, F3) | synthesized after click-to-focus: `key_actions=1` (F1 dump); F3 read host clipboard. F2/Q not separately exercised (same dispatch path) |
| Mouse events | PASS (click) | `mouse=1` on synthesized click at window center; drag/wheel/pointer-routing not covered |
| Chinese IME composition start/update | PASS | real Microsoft Pinyin (HKL 0x08040804): `G0 ime composition start` on first key, per-key updates with the marked range covering the whole composition (`text="n'ni'hao" marked_utf16=Some(108..116)`) and the selection advancing inside it (2026-08-23; `results/evidence/g0/pinyin-ime.log`) |
| Chinese IME commit + cancel | PASS | SPACE commit: `G0 ime commit text="拿你号" marked_replaced=144..152` (GCS_RESULTSTR arrives as `replace_text_in_range(None, …)` and replaces the marked span). ESC cancel: `G0 ime composition cleared (cancel path)`, selection returns to the composition start. Candidate docking observed via `bounds_for_range` calls on composition start |
| Clipboard write + read roundtrip | PASS | probe `roundtrip=ok token=markit-g0-probe-7625989` + external `Get-Clipboard` match + F3 read host-written `hello-from-host` |
| Demand redraw (notify → draws) | PASS | `G0 demand_1p2s notifies=42 draws=42 next_frame_callbacks=1` (2026-08-23 canonical; 2026-08-22 run: notifies=41 draws=15) |
| Idle behavior (draws ≈ 0, CPU) | PASS | passive 2 s: `draws=0 next_frame_callbacks=0` (no callback registered → nothing delivered); external: 2026-08-22 @100 % `cpu_s` frozen at 0.375 over 6 s (0.00 % CPU at rest), WS ~36 MB; 2026-08-23 @125 % `cpu_s` 0.344→0.406 (≈ 0.06 CPU-s / 6 s ≈ 1 % of one core), WS ~38 MB; with a self-sustaining `on_next_frame`: draws ≈ 0 while callbacks deliver at ~60/s (≈ every 60 Hz vsync) with the review-fixed binary at both scales — the superseded 2026-08-22 binary observed ~12.5/s; delivery rate is run-dependent, not a contract, and the platform vsync wake itself continues regardless (§4) |
| Startup sanity (first draw ms) | PASS | `G0 boot first_draw_ms=312.2` @100 % canonical (309.6 @125 %; 314.5 in the 2026-08-22 run; probe instrumentation; incl. device+font init) |
| Memory/RSS sanity | PASS | WS ~36 MB @100 % (2026-08-22 external sampling), ~38 MB @125 % (2026-08-23); 27–29 threads (idle, interactive run) |
| Input→frame instrumentation availability | PASS | pending-input seam measured action→paint 13.0 ms @100 % canonical / 6.29 ms (2026-08-22 run) — n=1 per run, treat as seam-availability evidence, not a latency statistic |

## 6. Update policy (changing the pin)

- Changing the GPUI rev is a **product dependency migration**, not a routine
  bump: it must rerun the applicable G0 Windows regression matrix (build,
  smoke matrix, idle behavior, IME/clipboard/fonts) and update §5.
- Do not track Zed `main`. Do not bump because a release is newer. A bump
  needs a reason that survives the evidence rules in AGENTS.md §3 (a Windows
  fix Markit needs, a measured blocker, or a security issue).
- The pin must always be an exact rev; PRs that replace rev with
  branch/tag/range are rejected by review.
- Markit carries no GPUI patches (§1). If a patch ever becomes necessary,
  record it here with: purpose, owner, rebase cost, and the upstream PR that
  obsoletes it.

## 7. Probe

`apps/markit` (feature `g0-probe`; since P0-03 it is a diagnostic on top
of the default `editor` product path, no longer the app's raison d'être)
contains the minimal probe used for §5:

```bash
# static gates from WSL (cross target): cargo check + clippy work cross-built
export PATH="/usr/lib/llvm-22/bin:$PATH"
export RC_x86_64_pc_windows_msvc="$PWD/tools/g0-xwin-llvm-rc.sh"
cargo xwin check -p markit --features g0-probe --target x86_64-pc-windows-msvc

# release build: run on the Windows host (see §5 — release cross-builds are
# unsupported under the current Linux-host path: gpui_windows' fxc shader
# step is host-cfg'd and its output include is release-only)
cargo build --release -p markit --features g0-probe

# run on the Windows desktop session
markit.exe --g0-probe --smoke     # scripted matrix → G0-prefixed trace, auto-quit
markit.exe --g0-probe             # interactive: F1 dump, F2/F3 clipboard, Q quit
```

`tools/g0-xwin-llvm-rc.sh` is a build-environment workaround: the pinned
GPUI's `gpui.rc` references its manifest relative to the crate root, which
MSVC `rc.exe` resolves via VS include paths but `llvm-rc` (used by
embed-resource on non-Windows hosts) does not. The wrapper re-anchors the
CWD before exec; no GPUI source is patched.

The probe is instrumentation, not an editor: no document model, no Markdown,
no scheduler policy. It is replaced by the P0-03 vertical slice.
