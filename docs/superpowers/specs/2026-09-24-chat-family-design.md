# 聊天一族组件设计（message / bubble / message_scroller / attachment / marker）

- 日期：2026-09-24
- 范围：在 `iced-kit`（基于 `iced` 0.14）中新建"聊天一族"五个组件，功能/视觉/API 语义对齐参考项目 `gpui-kit`（基于 `gpui`）。
- 交付目标：`MessageScroller` 完整对齐（虚拟化 + follow-tail 贴底 + 跳到底按钮 + 过渡 + 底部渐隐）；五组件全交互（回调消息）。

## 1. 背景与约束

参考实现位于 `gpui-kit/crates/component/src/`：`message.rs`、`bubble.rs`、`marker.rs`、`attachment.rs`、`message_scroller.rs`。gpui 与 iced 是两个不同的 UI 框架，因此本任务为**功能/视觉/API 语义的对齐移植**，不是代码移植。

gpui 中两个无对应物的机制，需转换为本项目惯用法：

1. `ParentElement` / `.child()` 收集子元素 → 使用 `Vec<Element>` + 链式 `.child(x)` / `.push()`（同 `sidebar.rs`）。
2. 全局 `Styled` trait（`.p_2()` / `.text_color()`）→ 不引入；builder 暴露语义化尺寸/间距方法，视觉一律走 `Theme::colors()` 语义 token（同 `details.rs`、`loading.rs`）。

## 2. 架构与模块布局

采用单一 `chat/` 子模块目录：

```
src/widgets/chat/
  mod.rs            // 重导出 + 构造器函数聚合
  message.rs        // MessageGroup, Message, MessageAvatar, MessageHeader,
                    //   MessageContent, MessageFooter, MessageAlignment
  bubble.rs         // Bubble, BubbleContent, BubbleGroup, BubbleReactions,
                    //   BubbleVariant, BubbleReactionSide
  marker.rs         // Marker, MarkerIcon, MarkerContent,
                    //   MarkerVariant, MarkerLoadingStyle
  attachment.rs     // Attachment, AttachmentMedia, AttachmentContent,
                    //   AttachmentTitle, AttachmentDescription,
                    //   AttachmentActions, AttachmentGroup, AttachmentStatus
  scroller.rs       // MessageScroller, MessageScrollerState
```

理由：五组件同属"聊天域"且互相引用（Message 内含 Bubble、Content 可含 Attachment/Marker），聚合在一起边界最清晰；`scroller` 单独成文件因其最重且依赖 `widgets::virtual_list`。

在 `src/widgets/mod.rs` 增加 `pub mod chat;` 并 `pub use chat::{...}` 重导出，与现有导出风格一致。

## 3. 复用的既有件

- `widgets::avatar::Avatar` —— Message 头像。
- `widgets::button::Button` —— Bubble reactions、Attachment actions。
- `widgets::shimmer`（`Shimmer` / `ShimmerStyle`）—— Marker `Shimmer` loading、Attachment 上传中扫光。
- `widgets::spinner::Spinner` —— Marker `Spinner` loading。
- `widgets::virtual_list`（`VirtualList` / `VirtualListState`）—— MessageScroller 的虚拟化、双端 spacer、滚动与测量。
- `crate::motion` / `widgets::reveal::Reveal` —— jump 按钮出现/消失过渡。
- `crate::theme::Theme::colors()` —— 全部语义色取色。

## 4. 各组件公开 API

统一惯用法：构造器函数（常见场景）+ `#[must_use]` builder（全选项），链式方法返回 `Self`，`impl Into<Element<'a, Message, Theme>>`。

### 4.1 Marker（会话状态/系统提示行，纯展示 + loading）

```rust
pub enum MarkerVariant { #[default] Plain, Separator, Border }
pub enum MarkerLoadingStyle { #[default] Spinner, Shimmer }

marker(content) -> Marker
Marker::new()
  .icon(MarkerIcon)
  .content(text)
  .loading(bool)
  .with_variant(MarkerVariant)
  .with_loading_style(MarkerLoadingStyle)
  .with_shimmer_style(ShimmerStyle)
  .separator_style(..)        // 语义化覆盖（可选）
  .child(..)
```

- 语义：`Plain` 行内无分隔；`Separator` 居中 + 两侧 `border` 色分隔线；`Border` 底部 `border` 色细线。
- `loading(true)`：`Spinner` → 行首旋转图标；`Shimmer` → 内容文字扫光。
- `MarkerIcon` / `MarkerContent` 为 child 收集型子容器；`MarkerContent.text(..)` 支持纯文本。
- 不主动产生交互消息，但保留 `Message` 泛型参数以统一组合。

### 4.2 Bubble（气泡，含表情回应）

```rust
pub enum BubbleVariant { #[default] Filled, Secondary, Muted, Tinted, Outline, Ghost }
pub enum BubbleReactionSide { Top, #[default] Bottom }

Bubble::new()
  .with_variant(BubbleVariant)
  .alignment(MessageAlignment)
  .content(BubbleContent)
  .reactions(BubbleReactions)
  .child(..)
BubbleContent::new().child(..)
BubbleGroup::new().child(..)
BubbleReactions::new().side(..).alignment(..).action(Button).child(..)
```

variant → 语义色映射（取自 `Theme::colors()`）：

| Variant | 背景 | 文字 | 边框 |
| --- | --- | --- | --- |
| Filled | primary | primary_foreground | 无 |
| Secondary | secondary | secondary_foreground | 无 |
| Muted | muted | muted_foreground | 无 |
| Tinted | primary（低不透明度） | foreground | 无 |
| Outline | surface | foreground | border |
| Ghost | 无 | foreground | 无 |

- reactions：`action(Button)` 接收本项目 `Button`，自带回调消息；`side` 决定在气泡上/下。

### 4.3 Message 族

```rust
pub enum MessageAlignment { #[default] Start, End }

MessageGroup::new().child(..)                 // 同一发件人连续消息竖向堆叠
Message::new()
  .alignment(MessageAlignment)
  .avatar(Avatar | Element)
  .avatar_slot(MessageAvatar)
  .header(MessageHeader)
  .content(MessageContent)
  .footer(MessageFooter)
  .child(..)
MessageContent::new().bubble(Bubble) | .child(..)
MessageAvatar / MessageHeader / MessageFooter // child 收集型容器
```

- `alignment == End`：整行右对齐、气泡靠右、头像落到结束侧；`Start` 反之。
- 头像复用 `widgets::avatar::Avatar`。
- `MessageContent` 除 `bubble` 外可 `child` 任意 Element（承载 Attachment 组等）。

### 4.4 Attachment（附件卡片，含上传状态）

```rust
pub enum AttachmentStatus { Pending, Uploading, Processing, Failed, #[default] Complete }
// 附带 is_pending / is_uploading / is_processing / is_failed / is_complete / is_in_progress

Attachment::new()
  .id(impl Into<ElementId>)
  .status(AttachmentStatus)
  .axis(Axis)
  .media(AttachmentMedia)
  .content(AttachmentContent)
  .actions(AttachmentActions)
  .on_click(f)                                 // 生成 Message
AttachmentMedia::new().src(source).overlay(el)
AttachmentContent::new().title(AttachmentTitle).description(AttachmentDescription)
AttachmentTitle::new(text).status(s)           // Uploading/Processing → shimmer
AttachmentDescription::new(text).status(s)
AttachmentActions::new().child(..)
AttachmentGroup::new(id).child(..)             // 卡片网格
```

- 状态视觉：`Failed` → `destructive`；`Complete` → `success` 勾；`Pending` → 静默；`Uploading`/`Processing` → 标题 shimmer + actions 区可放进度。
- `media.src` 用 `iced` 的 image/path source 抽象；无图时退化为文件类型图标。

### 4.5 MessageScroller（见第 5 段）

## 5. MessageScroller 行为与状态机

### 5.1 渲染结构（stack 叠加）

```
stack [
  VirtualList(...)        // 复用 widgets::virtual_list：plan/spacer/滚动/测量
  bottom_fade (可选)       // 顶部/底部渐隐遮罩，自定义绘制 widget
  jump_button (条件可见)   // "跳到底部"浮标
]
```

每条 message 为可变高一行；`row_height` 由调用方给估计值，实测走 `VirtualListState::measured`。不重造虚拟化逻辑。

### 5.2 状态模型（app 持有纯数据，与 VirtualListState 一致）

```rust
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MessageScrollerState {
    list: VirtualListState,   // 滚动偏移 + viewport + 测量
    follow_tail: bool,        // 是否贴底
    seen_len: usize,          // 上次 item 数，用于检测 append/prepend
}
```

对外方法：`is_following_tail()`、`is_scrolled_up()`、`scroll_to_end()`、`reset(len)`、`append(count)`、`prepend(count)`、`splice(range, count)`。均为纯函数式，返回是否需重绘。

### 5.3 状态机（每帧根据数据推导 + 回调维护）

1. **检测新消息**：`items.len() > seen_len` 且 `follow_tail == true` → 本帧保持贴底。`prepend`（历史加载）→ 不移动偏移，避免跳动。
2. **用户上滑**：`on_scroll` 中若 `offset < content_height - viewport - threshold`（`threshold ≈ 80px`）→ `follow_tail = false`；回到阈值内 → `follow_tail = true`。
3. **anchor 选择**：`follow_tail` 为真 → `scrollable.anchor(Anchor::BottomRight)`（引擎自动维持底部）；否则用 `viewport(VirtualOffset{ y: offset })` 精确还原。
4. **jump 按钮**：可见性 = `jump_button(true) && !follow_tail`。出现/消失用 `crate::motion`（`Reveal`/presence）淡入淡出。点击 → 发 `on_jump_to_bottom` 消息，app 将 state 置 `follow_tail = true; scroll_to_end()` 后塞回。

### 5.4 底部渐隐

自定义 `Widget`（参考 `color_picker.rs` 的 `Path` + fill 先例）画一条从透明 → `theme.colors().background` 的竖向线性渐变，高约 24–48px，叠在内容之上、按钮之下。`.with_bottom_fade(color)` 可覆盖颜色，默认取主题 background。若 iced 无便捷渐变图元，退化为多段半透明矩形叠加（同一 widget 内实现，不污染对外 API）。

### 5.5 Builder API

```rust
MessageScroller::new(items, &state, |item, index| Element)
  .scrollbar(bool)
  .jump_button(bool)
  .with_jump_button_label(impl Into<String>)
  .with_bottom_fade(impl Into<Color>)
  .on_scroll(f)                 // MessageScrollerState -> Message
  .on_jump_to_bottom(f)
  .width(..).height(..)
```

### 5.6 与 gpui 的差异（明示）

- 无 `Entity`/`cx`：改为纯数据 state + 回调。
- `with_*_style(StyleRefinement)` 系列不逐字移植；改为本项目语义 token 自动取色，仅保留少量必要覆盖（`with_jump_button_label`、`with_bottom_fade`、`scrollbar`）。

## 6. 测试策略

对齐项目现有 `#[cfg(test)]` 风格（参考 `virtual_list.rs`）：

- **纯函数单测**：`MessageScrollerState` 的 append/prepend/follow_tail 翻转、阈值判定、`seen_len` 推导（给定 offset/content_height/viewport 断言 `follow_tail`）。
- **枚举/映射测**：`AttachmentStatus::is_*` 全覆盖；`BubbleVariant` → 颜色映射稳定性。
- **渲染烟测**：每个组件 `.into()` 成 `Element<'_, Message, Theme>` 并 `drop`，遍历各 variant / alignment / loading / status 组合（仿 virtual_list 遍历参数组合构建 Element 的测法）。
- **示例画廊**：实现阶段在 `examples/` 增 `chat.rs`（仿 `gallery.rs` / `dock.rs`）供手动视觉核对。
- 不做 gpui 式快照测试（框架无对应设施）；视觉对齐靠示例 + 人工核对。

## 7. 非目标（YAGNI）

- 不实现 gpui 的 `StyleRefinement` 任意样式注入机制。
- 不实现附件真实上传/进度数据管线（仅状态视觉与回调）；进度数值由 app 提供文本/比例。
- 不实现 gpui 的 `AttachmentClickHarness` 专用测试脚手架（以 iced 交互测试替代）。
- 不引入消息列表数据的内部持有；`items` 始终由 app 传入。

## 8. 验收标准

- 五组件均可编译，导出至 `iced_kit::widgets::chat`（并在 `widgets::mod` 重导出）。
- `cargo test` 通过（含第 6 段所列单测与烟测）。
- `examples/chat.rs` 可运行，展示：左右对齐消息、头像、多种 bubble variant、reactions、marker 三形态与两种 loading、attachment 五状态、长对话列表的贴底/上滑/跳到底/底部渐隐。
- 视觉与语义与 `gpui-kit` 参考逐项对齐（差异仅第 5.6 / 第 7 段所列框架强制项）。

## 9. 实现偏差与有意简化（Task 1-6 定稿）

实现经两阶段代码审查后，以下相对本文档前述 API 的偏差被确认为**有意为之**，据 iced 0.14 约束或 YAGNI 保留，非缺陷：

- **`Attachment::id` / `AttachmentGroup::new(id)` 未实现**：本项目所用 iced 0.14 中 `iced::ElementId` 已更名为 `Id`，且 `button` 未提供链式 `.id()`（经核实）。可点击卡片依赖 iced 基于 widget `Path` 的稳定 id。若后续同构多卡出现命中冲突，再评估经 `iced::widget::Id` 手动注入。
- **`MessageScroller::scrollbar(bool)` 未实现**：`VirtualList` 未暴露滚动条开关，透传无实际效果，故省略（避免无效 API）。
- **`Marker::with_shimmer_style`、`MarkerIcon`/`MarkerContent` 独立容器**：shimmer 走默认样式；图标/正文以 `.icon(IconName)`/`.content(String)`/`.child(Element)` 内联，未拆成独立 child 收集容器。
- **`Message` 挂载方法命名**：`.header_el/.footer_el`（而非 `.header(MessageHeader)`）；未提供 `.avatar_slot()`，`MessageAvatar` 可经 `.avatar(impl Into<Element>)` 间接挂载。
- **签名微调**：`.with_bottom_fade(Option<Color>)`（需显式传色，不自动取 background）、`.on_jump_to_bottom(Message)`（携带一个消息值而非回调）。
- **跳到底落点**：新增 `MessageScrollerState::pin_to_tail()`（仅置 follow 标志，交 `anchor_bottom` 定位）作为按钮首选路径；`scroll_to_end(content_height, viewport)` 保留为已知尺寸的显式偏移路径。
- **底部渐隐**：以堆叠半透明带（顶部带全透明起、向下渐浓）实现，非自定义 `Widget` 渐变图元；jump 按钮为纯显隐（motion 过渡为可接受简化）。
