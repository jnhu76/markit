# mvp/ — Framework prototypes

`mvp/gpui` is the GPUI Windows feasibility prototype: a thin editable
text surface proving the direct-GPUI substrate for Markit (Phase A0,
`docs/phase-a0-windows-feasibility.md`).

It is now both:

```text
historical GPUI feasibility prototype
+
seed/reference for the direct-GPUI product foundation (ADR-008)
```

The prototype verified on Windows (gpui 0.2.2 — **not** the product
baseline, see roadmap G0):

- native window (Win32 + DirectX 11 + DirectComposition);
- Chinese text via DirectWrite fallback;
- pointer (click-to-cursor, drag selection, wheel scroll);
- keyboard (insert, backspace/delete, arrows, home/end, enter,
  ctrl+a/c/v/x);
- IME pipeline through gpui's input-handler contract (IMM32);
- resize + HiDPI;
- shared 7-stage trace contract and deterministic `--smoke` self-test.

The former `mvp/pocketjs` probe was removed on 2026-08-18 with the
architecture pivot to direct GPUI; its knowledge transfer record lives in
`docs/research/pocketjs-mvp-knowledge-transfer.md` and the source history.

## Run

```bash
cd gpui
cargo build --release
./target/release/mvp-gpui.exe --smoke   # deterministic self-test, auto-exits
```

See [mvp/gpui/README.md](gpui/README.md) for details and known constraints.

## GPUI baseline note

The prototype pins `gpui = "0.2.2"` (crates.io). This is the **existing
prototype version**, not the final product dependency policy. GPUI
baseline selection is the next experiment (roadmap G0); do not bump the
prototype version as part of product work without a baseline-selection
experiment.
