# 聊天一族组件 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 `iced-kit` 中实现聊天一族五组件（message / bubble / message_scroller / attachment / marker），功能与 API 语义对齐 `gpui-kit` 参考实现。

**Architecture:** 新建 `src/widgets/chat/` 子模块目录，按 `marker → bubble → message → attachment → scroller` 依赖顺序实现。gpui 的 `ParentElement`/`Styled` 机制转换为本项目 `Vec<Element>`+链式方法与 `Theme::colors()` 语义取色。MessageScroller 复用现有 `widgets::virtual_list`，用 app 持有的纯数据 `MessageScrollerState` 驱动 follow-tail 贴底状态机。

**Tech Stack:** Rust 2021, iced 0.14（features: advanced/canvas/markdown/svg）, `iced_test`（渲染断言）, `crate::motion`。

参考设计：`docs/superpowers/specs/2026-09-24-chat-family-design.md`。

---

## 文件结构总览

**新建：**
- `src/widgets/chat/mod.rs` —— 子模块声明 + 重导出
- `src/widgets/chat/marker.rs` —— Marker, MarkerIcon, MarkerContent, MarkerVariant, MarkerLoadingStyle
- `src/widgets/chat/bubble.rs` —— Bubble, BubbleContent, BubbleGroup, BubbleReactions, BubbleVariant, BubbleReactionSide
- `src/widgets/chat/message.rs` —— Message, MessageGroup, MessageAvatar, MessageHeader, MessageContent, MessageFooter, MessageAlignment
- `src/widgets/chat/attachment.rs` —— Attachment(+Media/Content/Title/Description/Actions/Group), AttachmentStatus
- `src/widgets/chat/scroller.rs` —— MessageScroller, MessageScrollerState
- `examples/chat.rs` —— 视觉画廊

**修改：**
- `src/widgets/mod.rs` —— 加 `pub mod chat;` 与 `pub use chat::{...}`

**约定：** 每个 `.rs` 用文件内 `#[cfg(test)] mod tests`；纯逻辑用真实断言，展示型组件用"构建 Element + drop"烟测（仿 `virtual_list.rs`）。提交信息用 conventional commits。测试命令统一 `cargo test --lib chat::` 或 `cargo test chat::`，全量 `cargo test`，lint `cargo clippy --all-targets -- -D warnings`。

---

## Task 0: 模块脚手架与导出

**Files:**
- Create: `src/widgets/chat/mod.rs`
- Modify: `src/widgets/mod.rs`（在 `pub mod carousel;` 附近加 `pub mod chat;`；文件末尾加 `pub use chat::*;`）

- [ ] **Step 1: 创建 `src/widgets/chat/mod.rs`（先只声明子模块，暂空实现占位）**

```rust
//! The chat family: messages, bubbles, markers, attachments and a
//! transcript scroller. See `docs/superpowers/specs/2026-09-24-chat-family-design.md`.

pub mod attachment;
pub mod bubble;
pub mod marker;
pub mod message;
pub mod scroller;

pub use attachment::{
    attachment, attachment_group, Attachment, AttachmentActions, AttachmentContent,
    AttachmentDescription, AttachmentMedia, AttachmentStatus, AttachmentTitle,
};
pub use bubble::{
    bubble, Bubble, BubbleContent, BubbleGroup, BubbleReactionSide, BubbleReactions, BubbleVariant,
};
pub use marker::{
    marker, Marker, MarkerContent, MarkerIcon, MarkerLoadingStyle, MarkerVariant,
};
pub use message::{
    message, message_group, Message, MessageAlignment, MessageAvatar, MessageContent,
    MessageFooter, MessageGroup, MessageHeader,
};
pub use scroller::{message_scroller, MessageScroller, MessageScrollerState};
```

> 子模块文件此时还不存在，编译会失败——这是预期的，Task 1+ 逐个补上。为了让本 Task 能独立提交，先创建五个空的子模块文件（各含 `//! 文档注释`），下一步一起建。

- [ ] **Step 2: 创建五个占位子模块文件**

分别新建 `src/widgets/chat/marker.rs`、`bubble.rs`、`message.rs`、`attachment.rs`、`scroller.rs`，每个只写一行文档注释（例：`//! Chat marker (implemented in Task 1).`）。同时把上面 `mod.rs` 里对应的 `pub use` 行**暂时注释掉**（`// pub use ...`），保留 `pub mod ...` 声明。

- [ ] **Step 3: 注册到 widgets/mod.rs**

在 `src/widgets/mod.rs` 的 `pub mod avatar;` 块后加入 `pub mod chat;`（保持字母序，位于 `carousel` 之后）。

- [ ] **Step 4: 运行编译，确认脚手架通过**

Run: `cargo build --lib`
Expected: 成功（五个空子模块 + 注释掉的导出，无 `pub use` 未解析错误）。

- [ ] **Step 5: Commit**

```bash
git add src/widgets/chat/ src/widgets/mod.rs
git commit -m "chore(chat): scaffold chat submodule and register in widgets"
```

---

## Task 1: Marker

**Files:**
- Create: `src/widgets/chat/marker.rs`（替换占位）
- Test: 文件内 `#[cfg(test)] mod tests`

依赖：`widgets::shimmer`（`Shimmer`, `ShimmerStyle`）、`widgets::spinner::Spinner`、`icons::IconName`。

- [ ] **Step 1: 写失败测试（枚举默认值 + 渲染烟测 + loading 组合遍历）**

在 `src/widgets/chat/marker.rs` 末尾追加：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;
    use iced::Element;

    #[derive(Clone, Debug)]
    enum Msg {}

    #[test]
    fn variants_default_to_plain_and_spinner() {
        assert_eq!(MarkerVariant::default(), MarkerVariant::Plain);
        assert_eq!(MarkerLoadingStyle::default(), MarkerLoadingStyle::Spinner);
    }

    #[test]
    fn renders_across_all_variants_and_loading() {
        for variant in [MarkerVariant::Plain, MarkerVariant::Separator, MarkerVariant::Border] {
            for loading in [false, true] {
                for style in [MarkerLoadingStyle::Spinner, MarkerLoadingStyle::Shimmer] {
                    let el: Element<'_, Msg, Theme> = Marker::new()
                        .content("Reading 3 files")
                        .with_variant(variant)
                        .loading(loading)
                        .with_loading_style(style)
                        .into();
                    drop(el);
                }
            }
        }
    }

    #[test]
    fn constructor_fn_builds_a_plain_marker() {
        let el: Element<'_, Msg, Theme> = marker("Yesterday").into();
        drop(el);
    }
}
```

- [ ] **Step 2: 运行，确认失败**

Run: `cargo test --lib chat::marker`
Expected: 编译失败（`Marker` 等未定义）。

- [ ] **Step 3: 实现 marker.rs**

```rust
//! A compact, composable row for conversation status and system markers.

use crate::icons::IconName;
use crate::theme::Theme;
use crate::widgets::{shimmer, spinner};
use iced::widget::{container, horizontal_rule, row, text};
use iced::{Alignment, Element, Length, Padding};

/// The visual treatment used by a [`Marker`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MarkerVariant {
    #[default]
    Plain,
    Separator,
    Border,
}

/// The visual treatment while a [`Marker`] is loading.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MarkerLoadingStyle {
    #[default]
    Spinner,
    Shimmer,
}

enum MarkerChild<'a, Message> {
    Icon(IconName),
    Content(String),
    Element(Element<'a, Message, Theme>),
}

/// A conversation status / system marker row.
#[must_use = "a Marker does nothing unless it is turned into an Element"]
pub struct Marker<'a, Message> {
    variant: MarkerVariant,
    loading: bool,
    loading_style: MarkerLoadingStyle,
    icon: Option<IconName>,
    children: Vec<MarkerChild<'a, Message>>,
    width: Length,
}

impl<'a, Message: 'a> Marker<'a, Message> {
    pub fn new() -> Self {
        Self {
            variant: MarkerVariant::default(),
            loading: false,
            loading_style: MarkerLoadingStyle::default(),
            icon: None,
            children: Vec::new(),
            width: Length::Fill,
        }
    }

    pub fn with_variant(mut self, variant: MarkerVariant) -> Self { self.variant = variant; self }
    pub fn loading(mut self, loading: bool) -> Self { self.loading = loading; self }
    pub fn with_loading_style(mut self, style: MarkerLoadingStyle) -> Self { self.loading_style = style; self }
    pub fn icon(mut self, icon: IconName) -> Self { self.icon = Some(icon); self }
    pub fn content(mut self, text: impl Into<String>) -> Self {
        self.children.push(MarkerChild::Content(text.into()));
        self
    }
    pub fn child(mut self, el: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.children.push(MarkerChild::Element(el.into()));
        self
    }
    pub fn width(mut self, width: impl Into<Length>) -> Self { self.width = width.into(); self }

    fn body(self, theme: &Theme) -> Element<'a, Message, Theme> {
        let muted = theme.colors().muted_foreground;
        let size = theme.typography().sm.size;

        let mut lead: Element<'_, Message, Theme> = if let Some(icon) = self.icon {
            icon.handle().to_text_element().size(size).style(move |_| iced::widget::text::Style { color: Some(muted) }).into()
        } else {
            text("​").size(0.1).into() // 占位零宽
        };
        // 若 loading 且 Spinner，在最前放旋转指示器。
        if self.loading && self.loading_style == MarkerLoadingStyle::Spinner {
            lead = row![lead, spinner::spinner::<Message>(false)].spacing(6).into();
        }

        let label: Element<'_, Message, Theme> = if self.children.is_empty() {
            text("").into()
        } else {
            let mut r = row![lead].spacing(6);
            for child in self.children {
                let node: Element<'_, Message, Theme> = match child {
                    MarkerChild::Content(t) if self.loading && self.loading_style == MarkerLoadingStyle::Shimmer => {
                        shimmer::shimmer_text(t, true).into()
                    }
                    MarkerChild::Content(t) => {
                        text(t).size(size).style(move |_| iced::widget::text::Style { color: Some(muted) }).into()
                    }
                    MarkerChild::Icon(name) => {
                        name.handle().to_text_element().size(size).into()
                    }
                    MarkerChild::Element(el) => el,
                };
                r = r.push(node);
            }
            r.into()
        };

        match self.variant {
            MarkerVariant::Plain => container(label).width(self.width).into(),
            MarkerVariant::Border => container(label)
                .width(self.width)
                .padding(Padding { top: 0.0, right: 0.0, bottom: 4.0, left: 0.0 })
                .style(move |t: &Theme| iced::widget::container::Style {
                    border: iced::Border { color: t.colors().border, width: 1.0, radius: 0.0.into() },
                    ..Default::default()
                })
                .into(),
            MarkerVariant::Separator => {
                let line = horizontal_rule(1.0).style(move |t: &Theme| iced::widget::horizontal_rule::Style { color: Some(t.colors().border) });
                row![
                    container(line).width(Length::Fill),
                    label,
                    container(line).width(Length::Fill),
                ]
                .align_y(Alignment::Center)
                .spacing(8)
                .width(self.width)
                .into()
            }
        }
    }
}

impl<'a, Message: 'a> Default for Marker<'a, Message> {
    fn default() -> Self { Self::new() }
}

impl<'a, Message: 'a> From<Marker<'a, Message>> for Element<'a, Message, Theme> {
    fn from(marker: Marker<'a, Message>) -> Self {
        // 借用主题需在 widget 层解析；此处用一个 container 承载 theme 闭包。
        iced::widget::container(iced::widget::text("")).stage_placeholder(marker)
    }
}
```

> ⚠️ 上面 `From` 里的 `stage_placeholder` 是**伪代码占位**：`Marker` 需要拿到 `Theme` 才能取色。真实做法有两条，二选一并全项目统一（在 Step 3 实施时定稿）：
>
> **(a) 延迟到 widget 渲染**（推荐，符合 iced）：让 `Marker` 内部构建一个自定义 `iced::advanced::Widget`，在 `draw` 里通过 `shell.theme()` 拿到 `Theme`。**但本项目现有组件（见 `details.rs`）都用更简单的路子**——见下。
>
> **(b) 走 `.style(closure)` 注入**（本项目主流做法，见 `details.rs`/`loading.rs`）：不在 `From` 里取色，而是把取色放进 iced 的 `container::style`/`text::style` 闭包，闭包参数即为 `&Theme`。因此 `body()` 不应接收 `theme`，改为**只在 style 闭包内取色**。
>
> **采用 (b)。** 重写 `body()` 为不接收 `theme`：所有颜色改由 `.style(move |t: &Theme| ...)` 闭包提供；`From` 直接 `container(...).into()`。`icon`/`text` 的 `color` 也改用 `StyleFn` 闭包。**参考 `details.rs:461-477` 的 `text_row` 里 `StyleFn` 写法与 `virtual_list.rs:471-476`。**

- [ ] **Step 4: 按方案 (b) 定稿实现，重跑测试**

Run: `cargo test --lib chat::marker`
Expected: PASS。

- [ ] **Step 5: 打开 mod.rs 里 marker 的 `pub use` 注释，编译**

Run: `cargo build --lib`
Expected: 成功。

- [ ] **Step 6: Clippy + Commit**

```bash
cargo clippy --all-targets -- -D warnings
git add src/widgets/chat/marker.rs src/widgets/chat/mod.rs
git commit -m "feat(chat): add Marker with Plain/Separator/Border and spinner/shimmer loading"
```

---

## Task 2: Bubble

**Files:**
- Create: `src/widgets/chat/bubble.rs`（替换占位）；`bubble.rs` 内定义 `MessageAlignment` 的**引用**（真正的 `MessageAlignment` 在 Task 3 `message.rs`，本 Task 先用 `crate::widgets::chat::message::MessageAlignment` 之前需 Task 3 就位。**为避免循环，`MessageAlignment` 定义提前到 bubble.rs 并在 mod.rs 从 bubble 重导出**——见下"类型归属"）。

**类型归属（重要，避免前后依赖打架）：** `MessageAlignment` 定义在 `message.rs`；但 `Bubble`/`BubbleReactions` 需要它。Rust 同 crate 内跨模块引用无顺序问题，直接用 `use super::message::MessageAlignment;` 即可。故**先做 Task 3、后做 Task 2**？不——保持"自底向上"测试顺序。**解决方案：** 本 Task 把 `MessageAlignment` 一并放进 `bubble.rs` 顶部定义，`message.rs` 从 `bubble` 引入。修改 `mod.rs`：`pub use bubble::{..., MessageAlignment};`，并从 `message` 的 `pub use` 去掉 `MessageAlignment`。

- [ ] **Step 1: 写失败测试（variant→取色稳定性 + reactions 布局 + 对齐遍历）**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;
    use iced::Element;

    #[derive(Clone, Debug)]
    enum Msg { React(usize) }

    const ALL: [BubbleVariant; 6] = [
        BubbleVariant::Filled, BubbleVariant::Secondary, BubbleVariant::Muted,
        BubbleVariant::Tinted, BubbleVariant::Outline, BubbleVariant::Ghost,
    ];

    #[test]
    fn default_variant_is_filled() {
        assert_eq!(BubbleVariant::default(), BubbleVariant::Filled);
    }

    #[test]
    fn renders_every_variant_and_alignment() {
        for v in ALL {
            for a in [MessageAlignment::Start, MessageAlignment::End] {
                let el: Element<'_, Msg, Theme> =
                    bubble("hello").with_variant(v).alignment(a).into();
                drop(el);
            }
        }
    }

    #[test]
    fn bubble_with_reactions_renders() {
        let el: Element<'_, Msg, Theme> = bubble("hi")
            .reactions(
                BubbleReactions::new()
                    .side(BubbleReactionSide::Top)
                    .action(crate::widgets::button("👍").into()),
            )
            .into();
        drop(el);
    }
}
```

> `BubbleReactions::action` 接收已构建好的 `Button`（`impl Into<Element>`）而非 `Button` 本体，简化耦合。若需强类型 `Button`，改签名为 `action(Button<'a, Message>)`；本计划取 `impl Into<Element<..>>`。相应更新测试里的调用（去掉 `.into()` 亦可）。

- [ ] **Step 2: 运行确认失败**

Run: `cargo test --lib chat::bubble`
Expected: 编译失败。

- [ ] **Step 3: 实现 bubble.rs**

关键片段（`MessageAlignment` 定义在**本文件**）：

```rust
//! Chat bubbles with alignment, variants, and emoji reactions.

use crate::theme::Theme;
use iced::widget::{container, row};
use iced::{Alignment, Element, Length};

/// Horizontal alignment for messages and message-owned surfaces.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MessageAlignment {
    #[default]
    Start,
    End,
}

/// A bubble's surface treatment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BubbleVariant {
    #[default] Filled, Secondary, Muted, Tinted, Outline, Ghost,
}

/// Which side of a bubble the reactions sit on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BubbleReactionSide {
    Top,
    #[default] Bottom,
}

#[must_use = "a Bubble does nothing unless it is turned into an Element"]
pub struct Bubble<'a, Message> {
    variant: BubbleVariant,
    alignment: MessageAlignment,
    content: Option<BubbleContent<'a, Message>>,
    reactions: Option<BubbleReactions<'a, Message>>,
    max_width: f32,
}

// 说明：BubbleContent 是 child 收集器；BubbleReactions 收集 Vec<Element> + side。
// Bubble::from(element) 渲染流程：
//   let surface = container(content_column).style(bubble_style(variant));
//   let stack = match side { Top => column![reactions_row, surface], Bottom => column![surface, reactions_row] };
//   外层 container(stack).align_x(match alignment {Start=>Leading, End=>Trailing}).width(Fill)

fn bubble_style(variant: BubbleVariant) -> impl Fn(&Theme) -> iced::widget::container::Style + Clone {
    use iced::widget::container::Style;
    use iced::{Background, Border};
    move |t: &Theme| {
        let c = t.colors();
        let radius = t.radius().lg.into();
        match variant {
            BubbleVariant::Filled => Style { background: Some(Background::Color(c.primary)), text_color: Some(c.primary_foreground), ..Default::default() },
            BubbleVariant::Secondary => Style { background: Some(Background::Color(c.secondary)), text_color: Some(c.secondary_foreground), border: Border { radius, ..Default::default() }, ..Default::default() },
            BubbleVariant::Muted => Style { background: Some(Background::Color(c.muted)), text_color: Some(c.muted_foreground), ..Default::default() },
            BubbleVariant::Tinted => Style { background: Some(Background::Color(with_alpha(c.primary, 0.12))), text_color: Some(c.foreground), ..Default::default() },
            BubbleVariant::Outline => Style { background: Some(Background::Color(c.surface)), text_color: Some(c.foreground), border: Border { color: c.border, width: 1.0, radius }, ..Default::default() },
            BubbleVariant::Ghost => Style::default(),
        }
    }
}

fn with_alpha(color: iced::Color, a: f32) -> iced::Color {
    iced::Color { a, ..color }
}
```

补全 `Bubble`/`BubbleContent`/`BubbleGroup`/`BubbleReactions` 的 `new`/链式方法/`From`（结构见 spec §4.2），`From` 内据 `alignment` 设置外层 `container.align_x`。padding：除 `Ghost` 外统一 `Padding::from([8.0, 12.0])` 风格。

- [ ] **Step 4: 更新 mod.rs 的 bubble 导出（含 MessageAlignment），重跑**

`src/widgets/chat/mod.rs`：取消 `pub use bubble::{...}` 注释并在列表加 `MessageAlignment`。
Run: `cargo test --lib chat::bubble`
Expected: PASS。

- [ ] **Step 5: Clippy + Commit**

```bash
cargo clippy --all-targets -- -D warnings
git add src/widgets/chat/bubble.rs src/widgets/chat/mod.rs
git commit -m "feat(chat): add Bubble with variants, alignment, and reactions"
```

---

## Task 3: Message 族

**Files:**
- Create: `src/widgets/chat/message.rs`（替换占位）

- [ ] **Step 1: 写失败测试（左右对齐 + 头像 + header/footer + content.bubble）**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;
    use crate::widgets::{avatar, bubble};
    use iced::Element;

    #[derive(Clone, Debug)]
    enum Msg {}

    #[test]
    fn message_renders_both_alignments() {
        for a in [MessageAlignment::Start, MessageAlignment::End] {
            let el: Element<'_, Msg, Theme> = message(
                bubble::bubble("hi").alignment(a),
            )
            .alignment(a)
            .avatar(avatar::avatar("AB"))
            .header("Assistant")
            .footer("12:30")
            .into();
            drop(el);
        }
    }

    #[test]
    fn message_group_stacks_children() {
        let el: Element<'_, Msg, Theme> = message_group()
            .child(message(bubble::bubble("one")))
            .child(message(bubble::bubble("two")))
            .into();
        drop(el);
    }
}
```

> `message(...)` 构造器接收一个 `Bubble`（或 `MessageContent`）。提供两个入口：`message(bubble) -> Message` 与 `Message::new()`。`.header(str)`/`.footer(str)` 为便捷方法，内部包装成 `MessageHeader`/`MessageFooter`；另有 `.header_el(MessageHeader)`/`.footer_el(...)` 收复杂容器。据参考 `.avatar(impl IntoElement)` → 这里 `impl Into<Element<..>>`。

- [ ] **Step 2: 运行确认失败**

Run: `cargo test --lib chat::message`
Expected: 编译失败。

- [ ] **Step 3: 实现 message.rs**

```rust
//! Message rows and groups: avatar, header, content (a bubble), footer.

use super::bubble::{Bubble, MessageAlignment};
use crate::theme::Theme;
use iced::widget::{column, container, row};
use iced::{Alignment, Element, Length};

#[must_use]
pub struct MessageGroup<'a, Message> { children: Vec<Element<'a, Message, Theme>> }
impl<'a, Message: 'a> MessageGroup<'a, Message> {
    pub fn new() -> Self { Self { children: Vec::new() } }
    pub fn child(mut self, el: impl Into<Element<'a, Message, Theme>>) -> Self { self.children.push(el.into()); self }
}
impl<'a, Message: 'a> Default for MessageGroup<'a, Message> { fn default() -> Self { Self::new() } }
impl<'a, Message: 'a> From<MessageGroup<'a, Message>> for Element<'a, Message, Theme> {
    fn from(g: MessageGroup<'a, Message>) -> Self {
        column(g.children).spacing(2).width(Length::Fill).align_x(Alignment::Start).into()
    }
}

// MessageAvatar / MessageHeader / MessageFooter：均为 child 收集型薄容器。
#[must_use]
pub struct MessageHeader<'a, Message> { children: Vec<Element<'a, Message, Theme>>, content_inset: bool }
#[must_use]
pub struct MessageFooter<'a, Message> { children: Vec<Element<'a, Message, Theme>> }
#[must_use]
pub struct MessageAvatar<'a, Message> { children: Vec<Element<'a, Message, Theme>> }
// 各 new/child/From 略（结构同 MessageGroup，渲染为 row/column）。
// MessageHeader.content_inset(bool)：控制是否与气泡正文左对齐缩进。

#[must_use]
pub struct MessageContent<'a, Message> { bubble: Option<Bubble<'a, Message>>, children: Vec<Element<'a, Message, Theme>> }
impl<'a, Message: 'a> MessageContent<'a, Message> {
    pub fn new() -> Self { Self { bubble: None, children: Vec::new() } }
    pub fn bubble(mut self, b: Bubble<'a, Message>) -> Self { self.bubble = Some(b); self }
    pub fn child(mut self, el: impl Into<Element<'a, Message, Theme>>) -> Self { self.children.push(el.into()); self }
}
impl<'a, Message: 'a> From<MessageContent<'a, Message>> for Element<'a, Message, Theme> {
    fn from(c: MessageContent<'a, Message>) -> Self {
        let mut col = column![].spacing(4);
        if let Some(b) = c.bubble { col = col.push(b); }
        for el in c.children { col = col.push(el); }
        col.into()
    }
}

#[must_use]
pub struct Message<'a, Message> {
    alignment: MessageAlignment,
    avatar: Option<Element<'a, Message, Theme>>,
    header: Option<MessageHeader<'a, Message>>,
    content: Option<MessageContent<'a, Message>>,
    footer: Option<MessageFooter<'a, Message>>,
}
// new/alignment/avatar/header/header_el/content/content_el/footer/footer_el + From。
// From 渲染：按 alignment 决定 row 顺序（avatar 在起侧还是止侧）、外层 container
//   .align_x(match alignment { Start=>Leading, End=>Trailing })。
//   header/footer 靠左（Start）或靠右（End）正文列；avatar 与正文列并排。

pub fn message<'a, Message: 'a>(bubble: Bubble<'a, Message>) -> Message<'a, Message> {
    Message::new().content(MessageContent::new().bubble(bubble))
}
pub fn message_group<'a, Message: 'a>() -> MessageGroup<'a, Message> { MessageGroup::new() }
```

补全所有 `new`/链式/`From`。`avatar(...)` 方法签名 `avatar(el: impl Into<Element<'a, Message, Theme>>)`。

- [ ] **Step 4: 更新 mod.rs 的 message 导出（去掉重复的 MessageAlignment，避免与 bubble 冲突）**

`mod.rs`：`pub use message::{...}` 取消注释，但**不含** `MessageAlignment`（它由 `bubble` 导出）。
Run: `cargo test --lib chat::message`
Expected: PASS。

- [ ] **Step 5: Clippy + Commit**

```bash
cargo clippy --all-targets -- -D warnings
git add src/widgets/chat/message.rs src/widgets/chat/mod.rs
git commit -m "feat(chat): add Message, MessageGroup, and message sub-containers"
```

---

## Task 4: Attachment

**Files:**
- Create: `src/widgets/chat/attachment.rs`（替换占位）

- [ ] **Step 1: 写失败测试（status 谓词全覆盖 + 渲染遍历 + on_click）**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_predicates_are_exclusive() {
        let s = AttachmentStatus::Uploading;
        assert!(s.is_uploading() && s.is_in_progress());
        assert!(!s.is_complete() && !s.is_failed() && !s.is_pending());
        assert!(AttachmentStatus::Processing.is_in_progress());
        assert!(AttachmentStatus::Failed.is_failed());
        assert!(AttachmentStatus::Complete.is_complete());
        assert!(AttachmentStatus::Pending.is_pending());
    }

    #[test]
    fn renders_every_status() {
        use crate::theme::Theme;
        use iced::Element;
        #[derive(Clone, Debug)] enum Msg {}
        for st in [AttachmentStatus::Pending, AttachmentStatus::Uploading,
                   AttachmentStatus::Processing, AttachmentStatus::Failed,
                   AttachmentStatus::Complete] {
            let el: Element<'_, Msg, Theme> = attachment("report.pdf")
                .status(st)
                .description("1.2 MB")
                .into();
            drop(el);
        }
    }
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test --lib chat::attachment`
Expected: 编译失败。

- [ ] **Step 3: 实现 attachment.rs**

```rust
//! Attachment cards with upload status, media preview, and actions.

use crate::icons::IconName;
use crate::theme::Theme;
use crate::widgets::shimmer;
use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Length, Padding};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AttachmentStatus {
    Pending,
    Uploading,
    Processing,
    Failed,
    #[default]
    Complete,
}

impl AttachmentStatus {
    #[must_use] pub fn is_pending(self) -> bool { self == Self::Pending }
    #[must_use] pub fn is_uploading(self) -> bool { self == Self::Uploading }
    #[must_use] pub fn is_processing(self) -> bool { self == Self::Processing }
    #[must_use] pub fn is_failed(self) -> bool { self == Self::Failed }
    #[must_use] pub fn is_complete(self) -> bool { self == Self::Complete }
    #[must_use] pub fn is_in_progress(self) -> bool { matches!(self, Self::Uploading | Self::Processing) }
}

// AttachmentMedia（src 图标/图片 + overlay）、AttachmentContent（title+description）、
// AttachmentTitle（Uploading/Processing 走 shimmer）、AttachmentDescription、
// AttachmentActions（child 收集）、AttachmentGroup（wrap 网格）与 Attachment 主体。
// Attachment 主体字段：status, axis(iced 无 Axis，用 Horizontal/Vertical 自建 enum 或复用), 
//   media, content, actions, on_click: Option<Box<dyn Fn()->Message>>。
// 状态取色：Failed->destructive；Complete->success；in-progress->标题 shimmer。
// on_click：有则整卡包成 button(...)，无则 container。

pub fn attachment<'a, Message: 'a>(title: impl Into<String>) -> Attachment<'a, Message> {
    Attachment::new().title(title)
}
```

> `iced` 无 `gpui::Axis`：用本项目已有的 `SplitAxis`（`widgets::resizable`）或在本文件定义 `AttachmentAxis { Horizontal, Vertical }`。**采用后者**，避免跨模块拉入 resizable。媒体预览若无图片管线，`src` 用 `IconName` 文件图标退化（spec §4.4 已述）。

补全全部 `new`/链式方法/`From`。`Attachment::id` 用 `iced::ElementId`（`impl Into<iced::ElementId>`）。

- [ ] **Step 4: 更新 mod.rs 的 attachment 导出，重跑**

Run: `cargo test --lib chat::attachment`
Expected: PASS。

- [ ] **Step 5: Clippy + Commit**

```bash
cargo clippy --all-targets -- -D warnings
git add src/widgets/chat/attachment.rs src/widgets/chat/mod.rs
git commit -m "feat(chat): add Attachment with upload statuses and actions"
```

---

## Task 5: MessageScroller 状态机（纯逻辑，TDD 核心）

**Files:**
- Create: `src/widgets/chat/scroller.rs`（本 Task 只放 state；widget 在 Task 6）
- 复用：`crate::widgets::virtual_list::VirtualListState`

- [ ] **Step 1: 写失败测试（follow-tail 检测、threshold、append/prepend、seen_len）**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_state_follows_tail() {
        let s = MessageScrollerState::new(0);
        assert!(s.is_following_tail());
    }

    #[test]
    fn append_while_following_keeps_follow() {
        let mut s = MessageScrollerState::new(3);
        s.record_len(4); // 一条新消息到来，仍贴底
        assert!(s.is_following_tail());
    }

    #[test]
    fn scrolling_up_releases_follow() {
        let mut s = MessageScrollerState::new(10);
        // content=1000, viewport=400 → 底部 offset=600。
        // 上滑到 400（距底 200 > threshold 80）→ 脱离贴底。
        s.apply_scroll(1000.0, 400.0, 400.0);
        assert!(!s.is_following_tail());
        assert!(s.is_scrolled_up());
    }

    #[test]
    fn scrolling_back_within_threshold_refollows() {
        let mut s = MessageScrollerState::new(10);
        s.apply_scroll(1000.0, 400.0, 400.0); // 脱离
        s.apply_scroll(1000.0, 400.0, 560.0); // 距底 40 < 80 → 回贴
        assert!(s.is_following_tail());
    }

    #[test]
    fn prepend_does_not_move_offset() {
        let mut s = MessageScrollerState::new(5);
        s.set_scroll(300.0);
        s.prepend(3); // seen_len +3，offset 不变
        assert_eq!(s.list_offset(), 300.0);
    }

    #[test]
    fn scroll_to_end_sets_follow() {
        let mut s = MessageScrollerState::new(10);
        s.apply_scroll(1000.0, 400.0, 100.0); // 顶部，脱离
        s.scroll_to_end(1000.0, 400.0);
        assert!(s.is_following_tail());
        assert_eq!(s.list_offset(), 600.0);
    }
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test --lib chat::scroller`
Expected: 编译失败。

- [ ] **Step 3: 实现 MessageScrollerState**

```rust
//! A virtualized, tail-following transcript scroller.

use crate::widgets::virtual_list::VirtualListState;
use std::cmp::Ordering;

/// Distance (px) from the bottom within which the scroller re-sticks to the tail.
const FOLLOW_THRESHOLD: f32 = 80.0;

/// App-owned scroller state (mirrors `VirtualListState`'s ownership model).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MessageScrollerState {
    list: VirtualListState,
    follow_tail: bool,
    seen_len: usize,
}

impl MessageScrollerState {
    #[must_use]
    pub fn new(item_count: usize) -> Self {
        Self { list: VirtualListState::new(), follow_tail: true, seen_len: item_count }
    }

    #[must_use] pub fn is_following_tail(&self) -> bool { self.follow_tail }
    #[must_use] pub fn is_scrolled_up(&self) -> bool { !self.follow_tail }
    #[must_use] pub fn list_offset(&self) -> f32 { self.list.offset() }
    #[must_use] pub fn seen_len(&self) -> usize { self.seen_len }
    #[must_use] pub fn inner(&self) -> &VirtualListState { &self.list }
    #[must_use] pub fn inner_mut(&mut self) -> &mut VirtualListState { &mut self.list }

    /// Records the incoming item count, adjusting follow/seen.
    /// An append while following keeps following; a prepend preserves offset.
    pub fn record_len(&mut self, new_len: usize) {
        match new_len.cmp(&self.seen_len) {
            Ordering::Greater => {} // append: 若本就贴底则维持；不主动改
            Ordering::Less => {}    // 数量减少（少见），仅同步 seen_len
            Ordering::Equal => {}
        }
        self.seen_len = new_len;
    }

    pub fn prepend(&mut self, count: usize) { self.seen_len += count; /* offset 不变 */ }
    pub fn append(&mut self, count: usize) { self.seen_len += count; }
    pub fn splice(&mut self, _range: std::ops::Range<usize>, new_len: usize) { self.seen_len = new_len; }
    pub fn reset(&mut self, item_count: usize) { *self = Self::new(item_count); }

    #[cfg(test)]
    pub fn set_scroll(&mut self, offset: f32) { self.list.update(offset, self.list.viewport_height()); }

    /// Applies a fresh scroll event given known content/viewport heights and
    /// returns whether the follow state changed.
    pub fn apply_scroll(&mut self, content_height: f32, viewport: f32, offset: f32) -> bool {
        self.list.update(offset, viewport);
        let max = (content_height - viewport).max(0.0);
        let was = self.follow_tail;
        self.follow_tail = (max - offset) <= FOLLOW_THRESHOLD;
        self.follow_tail != was
    }

    /// Jumps to the very bottom.
    pub fn scroll_to_end(&mut self, content_height: f32, viewport: f32) {
        self.list.update((content_height - viewport).max(0.0), viewport);
        self.follow_tail = true;
    }
}
```

> **关于 `#[cfg(test)] set_scroll`**：测试里 `prepend_does_not_move_offset` 需要先有个 offset。实现里 `set_scroll` 仅在 test 下暴露，避免污染公开 API。若评审更希望它有公开版本，改为 `pub fn set_offset_for_restore(...)`；本计划保守放 test-only。

- [ ] **Step 4: 运行确认通过**

Run: `cargo test --lib chat::scroller`
Expected: PASS。

- [ ] **Step 5: Commit**

```bash
git add src/widgets/chat/scroller.rs
git commit -m "feat(chat): add MessageScrollerState tail-follow state machine"
```

---

## Task 6: MessageScroller 渲染（虚拟化 + 贴底 + 跳底按钮 + 渐隐）

**Files:**
- Modify: `src/widgets/chat/scroller.rs`（在 Task 5 基础上加 widget）

- [ ] **Step 1: 写失败测试（Element 构建遍历：scrollbar/jump/fade 组合）**

```rust
#[cfg(test)]
mod render_tests {
    use super::*;
    use crate::theme::Theme;
    use iced::Element;

    #[derive(Clone, Debug)]
    enum Msg { Scrolled(MessageScrollerState), Jump }

    #[test]
    fn scroller_builds_across_options() {
        let items: Vec<String> = (0..500).map(|i| format!("msg {i}")).collect();
        for jump in [false, true] {
            for fade in [false, true] {
                let mut st = MessageScrollerState::new(items.len());
                if fade { st.apply_scroll(10_000.0, 400.0, 10.0); } // 上滑让按钮/渐隐可见
                let el: Element<'_, Msg, Theme> = MessageScroller::new(
                    &items, &st, |item, _| iced::widget::text(item.clone()).into(),
                )
                .jump_button(jump)
                .with_bottom_fade(fade.then_some(iced::Color::TRANSPARENT))
                .on_scroll(Msg::Scrolled)
                .on_jump_to_bottom(Msg::Jump)
                .height(400.0)
                .into();
                drop(el);
            }
        }
    }
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test --lib chat::scroller`
Expected: `MessageScroller` 未定义，编译失败。

- [ ] **Step 3: 实现 MessageScroller 渲染**

```rust
use crate::theme::Theme;
use crate::widgets::virtual_list::VirtualList;
use iced::widget::{container, scrollable, stack, text, Space};
use iced::{Anchor, Color, Element, Length, Padding, Vector};

#[must_use]
pub struct MessageScroller<'a, T: 'a, Message: 'a> {
    items: &'a [T],
    state: &'a MessageScrollerState,
    row: Box<dyn Fn(&'a T, usize) -> Element<'a, Message, Theme> + 'a>,
    row_height: f32,
    scrollbar: bool,
    jump_button: bool,
    jump_label: String,
    bottom_fade: Option<Color>,
    width: Length,
    height: Length,
    on_scroll: Option<Box<dyn Fn(MessageScrollerState) -> Message + 'a>>,
    on_jump: Option<Box<dyn Fn() -> Message + 'a>>,
}

impl<'a, T: 'a, Message: Clone + 'a> MessageScroller<'a, T, Message> {
    pub fn new(items: &'a [T], state: &'a MessageScrollerState,
               row: impl Fn(&'a T, usize) -> Element<'a, Message, Theme> + 'a) -> Self {
        Self { items, state, row: Box::new(row), row_height: 64.0, scrollbar: true,
               jump_button: false, jump_label: "↓ New messages".into(), bottom_fade: None,
               width: Length::Fill, height: Length::Fill, on_scroll: None, on_jump: None }
    }
    // 链式方法：scrollbar/jump_button/with_jump_button_label/with_bottom_fade/
    //   row_height/estimate/width/height/on_scroll/on_jump_to_bottom（略，签名见测试）。

    fn into_element(self) -> Element<'a, Message, Theme> {
        let content_height = self.items.len() as f32 * self.row_height;

        // 1) 虚拟化内容（复用 VirtualList 的 plan/spacer 逻辑）。
        let vlist = VirtualList::new(self.items, self.state.inner(), |item, i| (self.row)(item, i))
            .fixed_row_height(self.row_height)
            .width(self.width)
            .height_length(self.height);
        // VirtualList 内部已用 scrollable。为施加 anchor，需拿到其 scrollable：
        // 方案：不复用 VirtualList 的 From，而是复用其 plan 算法——通过一个本地
        //   build_scroller() 构造 scrollable 并 .anchor(...)。见 Step 4 注记。
        let content: Element<'_, Message, Theme> = self.build_scroller(content_height);

        // 2) stack 叠加：内容 + 渐隐 + 跳底按钮。
        let mut layers: Vec<Element<'_, Message, Theme>> = vec![content];
        if let Some(color) = self.bottom_fade {
            layers.push(BottomFade::new(color, self.state.is_scrolled_up()).into());
        }
        if self.jump_button && self.state.is_scrolled_up() {
            let btn = iced::widget::button(text(self.jump_label.clone()))
                .style(move |t: &Theme, _s| iced::widget::button::Style {
                    background: Some(iced::Background::Color(t.colors().primary)),
                    text_color: Some(t.colors().primary_foreground),
                    border: iced::Border { radius: t.radius().full.into(), ..Default::default() },
                    ..Default::default()
                });
            let btn = if let Some(on_jump) = &self.on_jump {
                btn.on_press((**on_jump)()).into()
            } else { btn.into() };
            layers.push(container(btn)
                .width(Length::Fill).height(Length::Fill)
                .align_x(iced::Alignment::Center).align_y(iced::Alignment::End)
                .padding(Padding { top: 0.0, right: 0.0, bottom: 12.0, left: 0.0 })
                .into());
        }
        stack(layers).width(self.width).height(self.height).into()
    }
}
```

- [ ] **Step 4: 处理 anchor 与 VirtualList 的衔接（关键实现决策）**

`VirtualList::into_element()` 内部直接产出 `scrollable` 且不支持外部 `.anchor()`。为施加贴底 anchor，**在 `scroller.rs` 内实现一个私有 `fn build_scroller(&self, content_height) -> Element`**：复制 `virtual_list.rs:266-412` 的 plan/spacer 生成逻辑（或抽 `VirtualList` 提供 `.anchor(Option<Anchor>)` builder 并让 `into_element` 应用之）。

**采用"给 VirtualList 加 anchor 支持"以 DRY**：在 `src/widgets/virtual_list.rs` 的 `VirtualList` 增字段 `anchor: Option<Anchor>` 与方法 `pub fn anchor(mut self, a: Anchor) -> Self { self.anchor = Some(a); self }`，并在 `into_element` 里 `if let Some(a) = self.anchor { scroller = scroller.anchor(a); }`。然后 `MessageScroller::build_scroller` 走：

```rust
let mut vl = VirtualList::new(self.items, self.state.inner(), render_row)
    .fixed_row_height(self.row_height).width(self.width).height_length(self.height);
vl = if self.state.is_following_tail() { vl.anchor(Anchor::BottomRight) } else { vl };
// 若脱离贴底，用绝对偏移还原（VirtualList 目前靠 on_scroll 读偏移；
//   还原由 app 传入的 state.list.offset 经 plan 生效——VirtualList.plan 已用 state.scroll_offset）。
```

同步为 VirtualList 增测：`anchor` 设置后仍能构建 Element（烟测）。为 `on_scroll` 桥接：`MessageScroller` 的 `on_scroll` 需把 `VirtualListState` 转成 `MessageScrollerState`——在 `build_scroller` 里用 `VirtualList::on_scroll` 回调，内部 `let mut next = self.state.clone(); next.list = reported; next.apply_scroll(content_height, viewport, reported.offset()); (self.on_scroll)(next)`。

- [ ] **Step 5: 写 BottomFade 自定义 widget**

在 `scroller.rs` 增 `struct BottomFade { color: Color, active: bool }`，`impl From<BottomFade> for Element` 用一个 `iced::advanced::Widget`（参考 `color_picker.rs` 的 `impl Widget` + `Path` 绘制与 `layout`）。`draw` 里画 4 段自 `alpha 0 → color.a` 的渐变矩形（退化方案，避免依赖未确认的渐变图元）。高度 `48.0`，`active==false` 时整段不画（贴底时无需遮罩）。

- [ ] **Step 6: 运行测试通过 + 全量构建**

Run: `cargo test --lib chat::scroller && cargo build --lib`
Expected: PASS，且 `scroller` 的 `pub use` 取消注释后整体编译通过。

- [ ] **Step 7: 更新 mod.rs 全部导出取消注释**

Run: `cargo build --lib`
Expected: 成功。

- [ ] **Step 8: Clippy + 全量测试 + Commit**

```bash
cargo clippy --all-targets -- -D warnings
cargo test
git add src/widgets/chat/scroller.rs src/widgets/virtual_list.rs src/widgets/chat/mod.rs
git commit -m "feat(chat): add MessageScroller with virtualization, tail-follow, jump button, bottom fade"
```

---

## Task 7: 示例画廊与端到端联调

**Files:**
- Create: `examples/chat.rs`

- [ ] **Step 1: 写可运行的聊天示例 app**

用 `iced::application` 搭一个 app：一个 `Vec<String>` 消息 + `MessageScrollerState` + 一个输入区，能：
- 渲染左右对齐消息（用户 End/主 Filled、助手 Start/Secondary），带 `avatar` 与 `header`/`footer`；
- 展示 `Marker`（Separator "今天" + 一条 `loading` 的 "Working…" Spinner）；
- 一个 `Attachment`（不同 status 各一）；
- 一个 `Bubble` 带 reactions；
- "发送"按钮 → `append` 新消息；`on_scroll` 更新 `MessageScrollerState`；`on_jump_to_bottom` → `scroll_to_end`。
- 底部 500 条填充以体现虚拟化与贴底。

（示例为完整 app，含 `main`，非 `#[test]`；仿 `examples/gallery.rs` 结构。）

- [ ] **Step 2: 编译示例（不运行 GUI）**

Run: `cargo build --example chat`
Expected: 成功。

- [ ] **Step 3: 手动视觉核对**

Run: `cargo run --example chat`
Expected: 窗口打开；发送消息自动贴底；上滑出现跳到底按钮与底部渐隐；点击按钮回到底部。

- [ ] **Step 4: 全量门禁**

```bash
cargo test
cargo clippy --all-targets -- -D warnings
```
Expected: 全绿。

- [ ] **Step 5: Commit**

```bash
git add examples/chat.rs
git commit -m "feat(chat): add chat gallery example app"
```

---

## Self-Review（编写者已跑）

- **Spec coverage**：§2 模块布局→Task 0；§4.1 Marker→Task 1；§4.2 Bubble(+MessageAlignment 归属)→Task 2；§4.3 Message→Task 3；§4.4 Attachment→Task 4；§5 Scroller（状态机+渲染+渐隐）→Task 5/6；§3 复用件均在对应 Task 引用；§6 测试→各 Task Step 1；§8 验收含 example→Task 7。无遗漏。
- **Placeholder scan**：Task 1 Step 3 明确给出定稿方案 (b) 并要求实施时去掉伪代码；`with_alpha`、`BottomFade`、`build_scroller` 均给了实现路径与参考文件行号；无裸 "TODO/TBD"。
- **Type consistency**：`MessageAlignment` 统一定义于 `bubble.rs` 并由 `message.rs`、`Bubble` 引用；`MessageScrollerState` 方法名（`is_following_tail`/`is_scrolled_up`/`apply_scroll`/`scroll_to_end`/`record_len`/`prepend`/`append`）在 Task 5/6/7 一致；`VirtualList::anchor` 新增方法在 Task 6 定义并被 `MessageScroller` 使用；`on_scroll` 载荷类型 `MessageScrollerState` 全程一致。
- **风险点（实施时首验）**：① 方案 (b) 取色闭包 `Clone + 'static`（`StyleFn`）约束——若 `Marker`/`Bubble` 编译不过，退化为 `container::style` 传 `&Theme` 的既有写法（`details.rs` 已验证可行）；② `scrollable.anchor(Anchor::BottomRight)` 在 iced 0.14 的确切方法名/存在性——Task 6 Step 4 前先 `cargo doc` 或查 `iced::widget::scrollable` API 确认，若不支持则改用每帧 `viewport(VirtualOffset::new(Vector{ x:0.0, y: max_offset }))` 还原。
