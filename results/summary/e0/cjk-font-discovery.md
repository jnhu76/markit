# E0 slice 1 — CJK fallback font discovery (evidence summary)

Date: 2026-08-17. Machine: win11_dt (Windows 11 + WSL2/WSLg, Ryzen 7 5800H).
Change: vendor/pocketjs@357f6fc (feat/cross-platform-cjk-fonts, PR jnhu76/pocketjs#5).

## Linux (WSLg) — real run, headless --screenshot

    note-widget: CJK fallback font "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc"#0
    note-widget: extended 7 font slot(s) with 19 new glyph(s)   # file load (cjk-test.md, 93 B)
    note-widget: extended 7 font slot(s) with 3 new glyph(s)    # typed 输入法

Screenshot (results/raw/e0/cjk-linux-wslg.png): full Chinese note rendered,
zero tofu. `cargo test -p note-widget`: 2/2 green.

## Windows — build + font chain verified; registry run pending

- `cargo xwin build -p note-widget --target x86_64-pc-windows-msvc` PASS (dev).
- `C:\Windows\Fonts\msyh.ttc` present (19.7 MB); faces 0/1 cover 中 and A
  through the same ab_glyph 0.2 parse path (probe over /mnt/c).
- Registry enumeration + render: PENDING (WSL interop disabled here;
  staged at C:\Users\fred1\AppData\Local\Temp\markit-e0, runbook in
  docs/phase-e0-desktop-enablement.md).

Matrix updates: Windows CJK FAIL→PARTIAL; Linux (WSLg) FAIL→PASS (WSLg-labeled).
