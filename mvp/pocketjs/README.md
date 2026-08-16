# mvp/pocketjs — Markit PocketJS thin editor MVP

PocketJS route of the Markit editor-substrate comparison. A Markit-owned
flat-widget host (Rust) + a Markit-owned thin-editor guest (SolidJS over
the PocketJS framework) that mirrors the GPUI Phase A0 prototype's window,
typography and editing semantics, so both MVPs edit the same corpus the
same way. Not an editor product, not a benchmark — the Phase A1 capability
probe (see `docs/phase-a1-pocketjs-windows.md`).

## Parity contract (vs `mvp/gpui`)

| Item | GPUI MVP | PocketJS MVP |
| --- | --- | --- |
| window | 1000x700 logical | 1000x700 logical |
| font | Consolas 18 px | Consolas 18 px (baked slot 3) |
| line height | 28 px | 28 px |
| soft wrap | none (long lines clip at the window) | none |
| visible lines | `floor(scroll/28) .. +ceil(vh/28)+1` | same |
| seed doc | `editor.rs` built-in | `app/sample.ts` (byte-identical) |
| palette | white / `#333333` / blue caret / `#3311ff30` sel | same |
| trace contract | 7 stages, `instrument.rs` | same names/format, `src/instrument.rs` |
| `--smoke` | deterministic steps + dump | same step order, minus IME |

## Layout

- `app/` — the guest: `editor.ts` (framework-free model), `app.tsx` (UI +
  input), `svc.ts` (host protocol), `sample.ts` (seed), `main.tsx` (entry)
- `src/` — the host: `main.rs` (boot, svc bridge, FlatWidget, smoke
  driver), `instrument.rs` (7-stage trace)
- `scripts/build-app.sh` — builds the guest bundle (bun build + Consolas)
- `dist/` — guest bundle output (gitignored)

## Build & run

```bash
scripts/build-app.sh                 # guest bundle → dist/
cargo build --release                # host (Linux/WSLg)
cargo xwin build --release --target x86_64-pc-windows-msvc   # Windows

./target/release/mvp-pocketjs            # interactive
./target/release/mvp-pocketjs --smoke    # deterministic self-test
./target/release/mvp-pocketjs --smoke --file ../../workloads/corpus/10k.txt
./target/release/mvp-pocketjs --file ../../workloads/corpus/10k.txt \
    --frames 120 --type "Hi!"@10 --scroll 56@40 --resize 800,700@50 \
    --screenshot /tmp/mvp.png            # headless (deterministic)
```

Controls: type to insert, arrows/backspace/delete/enter/home/end,
shift+arrows to select, ctrl+a select-all, ctrl+q quit, wheel to scroll.

## Instrumentation

Shared 7-stage contract with the GPUI MVP: `input_received`,
`edit_applied`, `layout_begin`, `layout_end`, `render_begin`,
`render_end`, `frame_submit`. Stage semantics on this host (proxies
explicit, never faked):

- `edit_applied` is a host-side proxy — the host records it when an edit
  is forwarded to the guest over svc; the guest applies it within the
  same tick (the mutation itself is not observable from the host).
- `frame_submit` is recorded after `queue.submit` in the windowed path;
  headless runs mark it `unavailable` (offscreen render, no present).
- Scripted (`--type`/`--key`/...) events count as `input_received` +
  `edit_applied` — they are the deterministic harness's stand-in for
  platform input, same spirit as GPUI's `--smoke` keystroke dispatch.

## Phase A1 status (MVP-side)

| Capability | Status |
| --- | --- |
| window launches (WSLg windowed) | PASS |
| seed text renders | PASS (CJK lines render tofu — no CJK system font discovery on PocketJS main; DEFERRED) |
| caret | PASS |
| typing | PASS |
| delete/backspace | PASS |
| arrows (+home/end, vertical) | PASS |
| selection | PASS (SelectAll; shift+arrow extends) |
| scroll | PASS |
| resize + relayout | PASS (svc-injected + real window path) |
| CJK | FAIL (tofu) — DEFERRED, capability item |
| IME | NOT TESTED (protocol reserved) — DEFERRED |
| clipboard | NOT TESTED — DEFERRED |
| Windows native run | PENDING (cross-build in progress; run + benchmark on the Windows host) |

Notes: the GPUI smoke drives IME composition via its input-handler path;
the PocketJS smoke skips IME steps (deferred). GPUI does not clamp its
manual scroll (a Phase A0 simplification); the PocketJS MVP clamps
correctly, so scroll smoke steps need a document taller than the viewport
(`--file workloads/corpus/10k.txt`).
