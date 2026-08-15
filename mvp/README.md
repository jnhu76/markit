# mvp/ — Framework MVP prototypes

统一管理 Markit 性能实验的框架可行性原型。每个 MVP 都是**能力探针**，不是产品、
不是 benchmark：同一套可控窗口（1000x700）、同一字体/字号/行高、同一 seed 文本，
配确定性 `--smoke` 自测和共享的 instrumentation 契约，为 Phase B 的
interaction-to-present 对比实验做准备。

| 框架 | 目录 | 状态 | 阶段报告 |
|---|---|---|---|
| GPUI | [`gpui/`](gpui/) | 原型完成（Phase A0 GO） | `docs/phase-a0-windows-feasibility.md` |
| PocketJS | [`pocketjs/`](pocketjs/) | **待做**（Phase A1 已在 fork 完成，见下） | `docs/phase-a1-pocketjs-windows.md` |

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

Phase A1（Windows desktop host 适配）已在 `jnhu76/pocketjs` fork 的
`support/windows-desktop` 分支完成并验证（thin editor MVP + 确定性 smoke +
instrumentation，`docs/phase-a1-pocketjs-windows.md`，状态 READY_FOR_PHASE_B）。
把该 MVP 落地到本目录（`mvp/pocketjs/`）是 Phase B 之前的下一个任务。
