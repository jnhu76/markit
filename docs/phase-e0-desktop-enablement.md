# Phase E0 — PocketJS Desktop Enablement / Windows Reference Host

Status: **IN PROGRESS** — first slice (cross-platform CJK font discovery)
delivered as `jnhu76/pocketjs#5` (draft); the phase continues with
clipboard and IME validation.

Roadmap position: A4 closeout → **E0 (this)** → P1 Windows Desktop MVP.
E0 proves the generic desktop capabilities *in PocketJS* so Markit P1
consumes them instead of growing a private OS layer
(`docs/product/architecture.md` §8).

## Slice 1 — CJK fallback font discovery (Windows/Linux)

### Question

The runtime font-atlas extension (`note-widget/src/cjk.rs`) only knew the
macOS font collections (`FONT_CANDIDATES`), so on Windows and Linux it
logged `no CJK-capable system font found` and every non-Latin codepoint
tofu'd (capability matrix: CJK fonts FAIL on Windows and WSLg — A1 gap).
Can discovery be made per platform, data-driven (no hardcoded font paths),
and still boot-time cheap?

### Setup

- Machine: win11_dt (Windows 11 + WSL2, AMD Ryzen 7 5800H); WSLg for the
  Linux side.
- Vendor change: `vendor/pocketjs` branch `feat/cross-platform-cjk-fonts`
  (commit 357f6fc, PR jnhu76/pocketjs#5):
  - Windows: enumerate `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\
    Fonts`, prefer CJK families, resolve files against `%SystemRoot%\Fonts`,
    verify coverage by parsing the face and checking the probe codepoint;
  - Linux/BSD: walk the standard font directories, prefer Noto CJK / WQY /
    Droid Sans Fallback by file name, verify coverage the same way;
  - macOS: unchanged.
- Guest: `dist/note-main.{js,pak}` built with the vendored pipeline
  (`bun tools/build.ts note-main`); host `cargo run -p note-widget`
  (dev profile), headless `--screenshot` mode.

### Workload

`cjk-test.md` — a 93-byte Markdown note with a Chinese heading, a
paragraph, and a two-item list (U1 simplified Chinese), loaded at boot
plus one scripted Chinese `--type` batch (kept in
`results/raw/e0/cjk-test.md`).

### Measurement

Linux (WSLg), `results/raw/e0/cjk-linux-wslg.{png,log}`:

```text
note-widget: CJK fallback font "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc"#0
note-widget: extended 7 font slot(s) with 19 new glyph(s)   ← file load
note-widget: extended 7 font slot(s) with 3 new glyph(s)    ← typed 输入法
```

Screenshot shows the full Chinese note rendered with zero tofu (every
character in the file is a real glyph).

Windows:

- Build: `cargo xwin build -p note-widget --target
  x86_64-pc-windows-msvc` — PASS (dev profile, commit 357f6fc).
- Font file: `C:\Windows\Fonts\msyh.ttc` (Microsoft YaHei, 19.7 MB) is
  present; faces 0 and 1 verified to cover 中 and A through the same
  ab_glyph 0.2 parse path the host uses (standalone probe over /mnt/c).
- Registry read + full render: **PENDING** — the dev WSL has interop
  disabled (`/etc/wsl.conf: interop enabled=false`), so the exe cannot be
  launched from WSL. Staged for a Windows-side run at
  `C:\Users\fred1\AppData\Local\Temp\markit-e0\` (exe + js/pak + test
  file); run `note-widget.exe --js note-main.js --pak note-main.pak
  --file cjk-test.md --frames 14 --screenshot cjk-win.png` there and
  check the log for the fallback-font line (expect msyh.ttc) and the PNG
  for tofu.

### Conclusion

CONFIRMED (Linux/WSLg): registry/dir-driven discovery selects a CJK font,
the atlas extension appends the missing codepoints, and Chinese renders
without tofu — the A1 gap's root cause was exactly the hardcoded macOS
candidate list.

SUPPORTED (Windows): the discovery code compiles for the MSVC target and
the font file + coverage chain are verified against the real system font;
the registry enumeration itself awaits a real Windows run (runbook above).

### Limitations

- The Linux evidence is WSLg — it certifies the host pipeline + discovery,
  not a real desktop install (P2).
- Emoji/COLR fallback is out of scope (Latin → CJK, same as before);
  the capability matrix tracks it separately.
- Boot-time discovery cost is one registry walk / directory scan + a few
  mmaps; not re-measured for startup regression (startup budget is a
  later E0/P1 item).

## Next slices (this phase, not started)

1. Windows real-machine run of the above (evidence → matrix cell PASS).
2. Text clipboard beyond pbcopy/pbpaste (Windows CF_UNICODETEXT; Linux
   wl-copy/xclip or a crate) — capability `host.clipboard`.
3. IME composition on Windows (pipeline exists: winit Ime → svc `{t:"ime"}`,
   caret docking; needs real Pinyin validation).
