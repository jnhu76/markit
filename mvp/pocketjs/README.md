# mvp/pocketjs — PocketJS thin editor MVP（待做）

**状态：未落地。** Phase A1（PocketJS Windows desktop host 适配）已完成于
`jnhu76/pocketjs` fork 的 `support/windows-desktop` 分支：thin editor MVP
（`apps/editor`）、确定性 `--smoke`、8 阶段 instrumentation 均在 fork 侧验证通过
（详见 `../../docs/phase-a1-pocketjs-windows.md`，状态 READY_FOR_PHASE_B）。

## 落地清单（Phase B 前）

- [ ] 从 `support/windows-desktop` 分支将 `apps/editor`（editor model + app +
      svc + sample）port 到本目录，与 `mvp/gpui/` 保持同一窗口/字体/字号/行高/
      seed 文本约定（1000x700、Consolas 16px、行高 28px、≥10 行含 ≥4 行中文）。
- [ ] 复用同款 `--smoke` 确定性自测与 trace schema（含 frame_submitted）。
- [ ] 人工验证 Microsoft Pinyin IME（组合 → 提交，候选窗口跟随 caret）。
- [ ] 记录 GPUI / PocketJS 两侧的 capability 对照，供 Phase B 实验设计使用。

## 相关产物

- fork 分支：`jnhu76/pocketjs` → `support/windows-desktop`
- 阶段报告：`docs/phase-a1-pocketjs-windows.md`
