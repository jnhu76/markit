# mvp/ — Framework MVP prototypes

统一管理 Markit 性能实验的框架可行性原型。每个 MVP 都是**能力探针**，不是产品、
不是 benchmark：同一套可控窗口（1000x700）、同一字体/字号/行高、同一 seed 文本，
配确定性 `--smoke` 自测和共享的 instrumentation 契约，为 Phase B 的
interaction-to-present 对比实验做准备。

| 框架 | 目录 | 状态 | 阶段报告 |
|---|---|---|---|
| GPUI | [`gpui/`](gpui/) | 原型完成（Phase A0 GO） | `docs/phase-a0-windows-feasibility.md` |
| PocketJS | [`pocketjs/`](pocketjs/) | 原型落地（Phase A1 MVP 侧 PASS，Windows 运行验证中） | `docs/phase-a1-pocketjs-windows.md` |

## 共享契约

两个 MVP 共用 trace schema：
`input_received / edit_applied / layout_begin / layout_end / render_begin / render_end / frame_submit`
（`frame_submit` 在 GPUI Windows spike 中不可观测，标记为 unavailable；PocketJS 侧已
通过 `frame_submitted` hook 补上）。

## 运行

```bash
cd gpui
cargo build --release
./target/release/mvp-gpui.exe --smoke   # 确定性自测，自动退出
```

## PocketJS MVP 状态

Markit 自有的 PocketJS thin editor MVP 已落地到本目录（`mvp/pocketjs/`）：
Markit-owned host（Rust）+ Markit-owned guest（SolidJS），与 GPUI MVP 同窗口/
同字体/同 seed/同 trace schema，P0-P9 在 WSLg 验证 PASS（CJK/IME/clipboard
DEFERRED）。PocketJS 以 submodule 形式固定在 `vendor/pocketjs`
（`feat/windows-mvp` 基线）。Windows 原生运行与 benchmark 是下一项。
