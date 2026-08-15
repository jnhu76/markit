# Markit Phase A0 — Windows GUI Feasibility Spike

**Status: READY_FOR_PHASE_A1**

Date: 2026-08-15
Scope: Windows 可行性 gate（Phase A 的 Windows 半边）。本阶段**不实现 Markdown 编辑器**，只验证两条 UI 路线（GPUI / PocketJS）在 Windows 上是否具备 thin editor MVP 的基础条件。

---

## 1. Environment

| 项 | 值 |
|---|---|
| OS | Windows 11 Pro 10.0.26200.9168 (x64) |
| Rust | rustc 1.96.0 (ac68faa20 2026-05-25), stable-x86_64-pc-windows-msvc |
| MSVC | VS 2022 Community, VC Tools 14.44.35207 |
| Windows SDK | 10.0.26100.0 |
| GPU | AMD Radeon(TM) Graphics（集成显卡，DirectX 12 可用） |
| 显示器 | 2560x1440, scale factor 1.0 (100%) |
| 仓库状态 | 审计开始时为空仓库（无 commit、无 README、无代码） |

实验设计文档 `Markit_Phase0_GPUI_PocketJS_实验设计_v0.1.docx` 位于 `~/Downloads/`（仓库中无 Markdown 版），已按其中 Phase A gate 执行 Windows 侧验证。

## 2. GPUI result — **GO（Windows 可行性成立）**

**依赖**：crates.io `gpui = "0.2.2"`（Zed Industries）。Windows 后端为完整原生实现：Win32 窗口 + DirectX 11 渲染（DirectComposition）+ DirectWrite 文本 + IMM32 IME。

### 2.1 验证矩阵（Windows 实测）

| 能力 | 状态 | 证据 |
|---|---|---|
| Native window | ✅ | 1000x700 窗口创建，标题、bounds、scale_factor 日志 |
| 固定字体文本 | ✅ | Consolas 18px，行距 28px，像素级截图验证 11 行布局 |
| 常规中文显示 | ✅ | 种子文档含 4 行中文；字形密度分析确认正常笔画（非 tofu） |
| Mouse input | ✅ | 注入 WM_LBUTTONDOWN → 点击定位光标 + focus |
| Keyboard input | ✅ | 注入 WM_CHAR（ASCII+中文）→ 文本插入（截图像素对比验证） |
| Insert/delete/cursor | ✅ | smoke 全链路：插入/IME/backspace/enter/end/home 均生效 |
| Selection | ✅ | shift+箭头 / cmd-a（smoke 验证 selection=0..len） |
| Scroll | ✅ | wheel 事件路径 + 程序化 scroll（smoke 验证 scroll_y 变化） |
| Resize | ✅ | window.resize + 拖拽 resize → bounds observer 日志 |
| HiDPI 基础 | ✅ | scale_factor/rem_size 可查询；DPI-aware 由 gpui 处理 |
| IME（中文输入） | ⚠️ 低成本验证 | 见 2.3 |
| Clipboard | ✅ | 代码路径存在（windows/clipboard.rs）+ smoke 的 copy/paste action |
| Instrumentation | ⚠️ 6/7 阶段 | frame_submit 暂不可观测，见 2.4 |

### 2.2 关键实现结论

- **输入管线**：未绑定按键的 `key_char` 直达聚焦元素的 input handler（`EntityInputHandler`），与 IME 共用同一路径；action（backspace/enter/箭头等）走 keybinding 分发。
- **两个必须知道的用法约束**（踩坑记录，已修复）：
  1. `cx.bind_keys` 必须在 `open_window` **之前**调用，否则 keymap 为空、所有键盘 action 失效（官方 `examples/input.rs` 即此顺序）。
  2. `window.focus` 受 `focus_enabled` 门控：**窗口未激活时 focus 被静默忽略**；input handler 只在元素聚焦时注册。真实用户点击窗口即可（验证通过）；自动化注入需先激活窗口。
- **初始 focus**：窗口打开后需显式 focus 编辑器（否则键盘输入全部丢弃）。

### 2.3 IME 状态

- GPUI Windows 后端实现了 **IMM32** 路径：`WM_IME_STARTCOMPOSITION`（设置组合窗口/候选窗口位置）、`WM_IME_COMPOSITION`（组合串/结果串）、`ImmGetVirtualKey`（组合期间按键）。代码证据：`src/platform/windows/events.rs`。
- 应用层 IME 契约完整：`replace_and_mark_text_in_range`（组合）、`marked_text_range`、`unmark_text`、`bounds_for_range`（组合窗口定位）在 smoke 中验证通过。
- **未验证**：真实中文输入法的端到端组合（需要人工键盘输入；自动化注入的 KEYEVENTF_UNICODE 不经过 IME 组合路径）。IMM32 对微软拼音等传统 IME 可用；TSF-only 输入法（部分新式 IME）可能不兼容 —— **Phase A1 需人工验证**。

### 2.4 Instrumentation skeleton

`src/instrument.rs` 实现共享契约的 7 阶段环形缓冲：`input_received / edit_applied / layout_begin / layout_end / render_begin / render_end / frame_submit`。

- **6/7 可观测**（smoke 记录：input=8, edit=6, layout=6, render=6）。
- **frame_submit = unavailable**：`Window::on_next_frame` 回调在本 spike 的 Windows 运行时从未触发（已用独立回调验证；draw 正常发生——layout/render 每帧记录）。present 前一刻在应用层不可观测。这是 framework 的 instrumentation 边界，Phase B benchmark 需要其他方案（如 ETW/DXGI present 挂钩）。

### 2.5 构建与运行

```bash
cd mvp/gpui
cargo build                 # debug
cargo build --release       # release（已验证）
./target/release/mvp-gpui.exe            # 交互运行
./target/release/mvp-gpui.exe --smoke    # 确定性自测（编辑→IME→action→scroll→resize→trace dump→退出）
```

## 3. PocketJS result — **CONDITIONAL GO（Windows 需自担 host 补齐，本阶段不实现）**

审计基于 `pocket-stack/pocketjs` master（2026-08-15 克隆，90MB，1292 文件）。

### 3.1 已具备（Windows 可直接复用）

- **编译**：desktop 链（`pocket3d` + `pocket-ui-wgpu` + `pocket-widget`）在 windows-msvc 上 **cargo check 通过**（本次实测，2m01s）——winit 0.30 + wgpu 25 无编译障碍。
- winit 窗口/事件循环/IME（TSF via winit）/HiDPI 管道（跨平台）。
- 完整 runtime 栈：`pocket-mod`（QuickJS guest）→ `pocketjs-core` → draw list → wgpu 渲染。
- guest 编辑器能力已在 JS 层实现（`apps/note`：软换行、caret、selection、undo/redo、IME 协议），有 bun 单测。
- headless 确定性渲染（CI 依赖）。

### 3.2 缺失（Windows 专属）

| 缺失 | 层次 | 证据 |
|---|---|---|
| Windows clipboard | host | `note-widget/src/main.rs`：`#[cfg(target_os="macos")]` 用 pbcopy/pbpaste；非 macOS 仅 `log::warn!` + None |
| Windows 系统字体路径 | host | `note-widget/src/cjk.rs`：`FONT_CANDIDATES` 硬编码 5 个 macOS 路径；Windows 上中文会 tofu |
| 快捷键语义 | host | ⌘ 硬编码（`super_down()`）；Windows 上 ⌘Q/⌘C/⌘V/undo 全部失效 |
| Windows target identity | contract | `platforms.ts` 只有 psp/vita/macos-widget 三个 target；`set_identity("macos-widget", 3)` 硬编码 |
| Windows CI / 实测 | 工程 | CI 全 ubuntu-latest；上游从未在 Windows 编译/运行过 desktop sample |
| 发布形态 | 工程 | 5 个 desktop crate 均未发布 crates.io（HTTP 404），只能 git dependency |

### 3.3 架构性限制（非 Windows 特有，但影响 Markit）

- core 无运行时 shaping（预烘焙 atlas 查表），无 kerning/ligature/复杂脚本；**无自动换行**（core 注释明示 v1 限制），换行逻辑在 guest JS。
- 大文档的 `measureText` 全量测量 + draw list 全量重发，与 Markit 低延迟目标的匹配度未经验证（Phase B 需测）。

### 3.4 最小补齐范围（若放行）

约数百行 + 一个 CI job：① platforms.ts 增 windows target + identity 参数化；② clipboard 换跨平台实现（arboard/windows API）；③ cjk.rs 增 Windows 字体路径（或 fontdb）；④ 快捷键按平台分支；⑤ windows CI job；⑥ 验证 winit0.30+wgpu25+rquickjs0.12 在 windows-msvc 的完整构建（本次仅 check 通过）。

### 3.5 判定

- **是否把 Markit 带入 framework infrastructure 工作**：**是**。upstream 明确只承诺 macOS 桌面（README / WIDGET.md / platforms.ts），Windows 支持须由 Markit 自担并维护 fork。
- **推荐：CONDITIONAL GO**。技术可行性成立（编译实测通过、补齐范围边界清晰），但前提是接受自担 host 工作。若 Phase A1 的 1–2 周 spike（完整构建 + 窗口运行 + 中文 IME + 10k 文档帧预算）失败，降级 NO-GO。
- **本阶段不实现 prototype**：按任务停止条件（无可用/已验证的 Windows host path → STOP，不自行实现 Windows backend）。

## 4. Windows support matrix

| 能力 | GPUI 0.2.2 | PocketJS master |
|---|---|---|
| 窗口/事件循环 | ✅ Win32 原生 | ⚠️ winit 跨平台（macOS 验证，Windows 未验证） |
| 渲染 | ✅ DirectX 11 + DirectComposition | ⚠️ wgpu 25（Windows 可编译，未运行验证） |
| 文本 shaping | ✅ DirectWrite + fallback | ❌ 预烘焙 atlas，无运行时 shaping |
| 中文显示 | ✅ 实测 | ❌ Windows 字体路径缺失（tofu） |
| IME | ⚠️ IMM32（代码完整，未人工验证） | ⚠️ winit TSF 管道（未实测） |
| Clipboard | ✅ | ❌ macOS-only |
| HiDPI | ✅ scale_factor 可查询 | ⚠️ winit ScaleFactorChanged |
| crates.io 发布 | ✅ 0.2.2 | ❌ git dependency only |

## 5. Missing capabilities

1. **GPUI**：真实 IME 组合人工验证（IMM32 对 TSF-only IME 的兼容性）；`frame_submit` 观测点（应用层不可得，需 ETW/DXGI 方案）。
2. **PocketJS**：见 3.2 的 6 项缺失（clipboard/字体/快捷键/identity/CI/发布）。

## 6. Scope risks

1. **PocketJS 的 host 补齐工作**：若 Phase A1 spike 中 winit/wgpu/QuickJS 在 Windows 出现环境性阻塞（如驱动/后端枚举问题），或 CJK/IME 管道在 Windows 走不通，则 CONDITIONAL GO 失效 → NO-GO。
2. **GPUI IMM32**：现代 Windows 输入法多为 TSF；IMM32 兼容性（微软拼音可用，部分第三方 TSF-only IME 不可用）需人工验证。若不可用，GPUI 需要 TSF 工作（上游未实现）——这是 Phase A1 的人工验证项。
3. **GPUI 0.2.2 平台后端成熟度**：`on_next_frame` 未触发、focus_enabled 门控等行为表明 Windows 后端仍需实战打磨；Phase B benchmark 可能遇到更多平台层问题（这本身是可行性证据的一部分）。
4. 本 spike 仅覆盖 Windows；Linux 侧（Phase A 的另一半）未验证。

## 7. Verdict

- **GPUI: GO**（Windows thin editor MVP 基础条件成立；2 个已知用法约束已文档化，1 个人工验证项）
- **PocketJS: CONDITIONAL GO**（编译可行，host 补齐范围明确；需 Markit 自担 framework infrastructure 工作，Phase A1 先做 1–2 周 spike 验证四个关键假设）

**Phase A0 最终状态：READY_FOR_PHASE_A1**

## 8. 下一步建议

1. **GPUI 人工验证**（10 分钟）：运行 `mvp-gpui.exe`，用微软拼音输入中文，验证 IMM32 组合路径；切英文输入法验证 ASCII。
2. **PocketJS Phase A1 spike**（1–2 周）：engine workspace 完整 windows-msvc 构建 → `note-widget` 窗口化运行 → 补齐 clipboard/字体/快捷键/identity → 中文 IME 走通 → 1–2 万字符文档的 60Hz tick 帧预算。
3. **统一 trace schema**：两个 MVP 共用 `input_received / edit_applied / layout_begin / layout_end / render_begin / render_end / frame_submit` 契约（`mvp/gpui/src/instrument.rs` 已实现）。
4. **公平性基线**：Phase B 前锁定窗口尺寸（1000x700）、字号（18px）、行高（28px）、字体（Consolas + CJK fallback）、corpus（plain-10k 风格）。

## 9. 实际运行命令（全部可重复）

```bash
# 环境
rustc --version                     # 1.96.0
cargo --version                     # 1.96.0

# GPUI prototype
cd mvp/gpui
cargo build                         # debug 构建（首次 ~3-4 min）
cargo build --release               # release 构建（已验证，3m45s）
./target/release/mvp-gpui.exe       # 交互运行（窗口 + 中文文本 + 输入）
./target/release/mvp-gpui.exe --smoke   # 确定性自测（自动退出）

# PocketJS 编译验证（audit 用）
git clone --depth 1 https://github.com/pocket-stack/pocketjs /tmp/pocketjs-audit
cd /tmp/pocketjs-audit/engine
cargo check -p pocket3d -p pocket-widget -p pocket-ui-wgpu   # 2m01s, exit 0
```

## 10. Commits / dependency versions

- 本仓库首个 commit 由本 spike 产生（见 git log）。
- `gpui = "0.2.2"`（crates.io），`unicode-segmentation = "1.13.3"`。
- PocketJS：`pocket-stack/pocketjs` master @ 2026-08-15（shallow clone，无固定 commit；依赖方式为 git path dependency，未发布 crates.io）。
- Rust stable 1.96.0 / MSVC 14.44 / WinSDK 10.0.26100 / Windows 11 26200.9168。
