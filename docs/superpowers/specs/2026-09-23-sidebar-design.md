# Sidebar 组件设计（对齐 gpui-kit）

日期：2026-09-23
状态：待实现

## 目标

把 `gpui-kit/crates/component/src/sidebar`（mod/menu/group/header/footer 五个
文件）对齐到 iced-kit，API 形态与参考实现一一对应，行为差异仅来自 iced 的
无状态约定。

参考实现的行为清单：

- 容器 `Sidebar`：header/content/footer 三段，`side`、`collapsible`、
  `collapsed`、像素宽度（默认 255）。
- 三种折叠模式：`Icon`（收窄到 48 的图标条）、`Offcanvas`（宽度动画到 0 并
  释放布局）、`None`（忽略折叠态）。
- `SidebarGroup`：分组标题，Icon 折叠时隐藏。
- `SidebarMenu` / `SidebarMenuItem`：图标、active、disabled、递归子菜单、
  suffix、折叠时以图标居中并在悬停时显示名称 tooltip、右键菜单。
- `SidebarHeader` / `SidebarFooter`：可选中/可悬停的插槽，带下拉菜单回调。
- `SidebarToggleButton`：按 side × collapsed 选择
  `PanelLeft/Right × Open/Close` 四个图标。
- 折叠/展开的宽度过渡动画。

## 范围外

- 可拖拽调宽（参考实现同样不支持）。
- 响应式断点、移动端抽屉形态。
- 独立的 `SidebarRail`（gpui-kit 主干没有，YAGNI）。

## 模块结构

新增 `src/widgets/sidebar.rs` 单文件（iced 侧无需 gpui-kit 的多文件拆分：
没有独立的 style trait 实现）。在 `src/widgets/mod.rs` 导出：

```rust
pub use sidebar::{
    sidebar, Sidebar, SidebarCollapsible, SidebarFooter, SidebarGroup,
    SidebarHeader, SidebarMenu, SidebarMenuItem, SidebarSide, SidebarToggleButton,
};
```

`sidebar(...)` 是与 `Sidebar::new()` 等价的构造函数，遵循本仓库
"模块 + 同名构造函数" 的约定。

### 类型与 builder

```rust
pub enum SidebarCollapsible { Icon, Offcanvas, None }  // From<bool>: true→Icon

pub enum SidebarSide { Left, Right }                   // 默认 Left，sidebar 自带：
                                                       // 复用 DrawerSide 会带上不适用
                                                       // 的 Bottom 且默认是 Right

pub struct Sidebar<'a, Message> { /* ... */ }
impl Sidebar<'a, Message> {
    fn new() -> Self;
    fn side(self, SidebarSide) -> Self;
    fn collapsible(self, impl Into<SidebarCollapsible>) -> Self;
    fn collapsed(self, bool) -> Self;
    fn width(self, f32) -> Self;              // 像素宽，默认 255
    fn header(self, SidebarHeader<'a, Message>) -> Self;
    fn footer(self, SidebarFooter<'a, Message>) -> Self;
    fn child(self, impl SidebarItem<'a, Message>) -> Self;
    fn children(self, impl IntoIterator<Item = impl SidebarItem<'a, Message>>) -> Self;
}

pub trait SidebarItem<'a, Message>: Sized {
    fn into_element(self, collapsed: bool) -> Element<'a, Message, Theme>;
    fn collapsed(self, collapsed: bool) -> Self;  // 容器逐层下推
}

pub struct SidebarGroup<'a, Message>;   // new(label).child(..)/children(..)
pub struct SidebarMenu<'a, Message>;    // new().child(..)/children(..)
pub struct SidebarMenuItem<'a, Message>;
// new(label)
//   .icon(IconName) .active(bool) .disabled(bool)
//   .children(..)                 // 递归子菜单
//   .open(bool) .on_toggle(M)     // 子菜单展开态与切换消息
//   .on_select(M)                 // 点击（disabled 时不发）
//   .suffix(Element)              // 徽章/开关
//   .context_menu(builder)        // 右键菜单内容
//   .tooltip_text(...)            // 折叠态悬停提示

pub struct SidebarHeader<'a, Message>;  // new().child(..) + dropdown 回调
pub struct SidebarFooter<'a, Message>;  // 同上
pub struct SidebarToggleButton<Message>; // .side(..).collapsed(..).on_press(M)
```

### 与 gpui-kit 的三处差异（均为 iced 无状态约定）

1. 闭包回调 `on_click(|ev, window, cx| ..)` → 消息 `on_select(Message)`。
2. keyed state 持有的子菜单展开 → 显式 `.open(bool)` + `.on_toggle(Message)`；
   `click_to_open` 不单设开关，`.open(true).on_toggle(..)` 可表达（YAGNI）。
3. `Styled` 链 → `.width(f32)`：宽度是 sidebar 唯一真正需要的样式项。

## 状态与消息流

全部无状态，调用方持有：`collapsed: bool`、active 标识、
`open: 集合<标识>`、浮层打开状态（走已有 `overlay::Layer`）。

- 容器算出 `icon_collapsed = collapsed && collapsible == Icon`，经
  `SidebarItem::collapsed(bool)` 下推给 group（隐藏标题）、menu、item
  （图标居中 + tooltip）。
- `Offcanvas` 折叠时容器宽度动画到 0、内容被裁剪。
- 右键菜单、header/footer 下拉、折叠 tooltip：组件保存菜单内容构造器并
  报告意图，是否显示与挂载位置由应用的 `overlay::Layer` 决定，与现有
  `DropdownButton`/`ContextMenu` 一致。
- 无 I/O、无 panic 路径；空组、越界 active 渲染为空而非 panic（同 tabs）。

## 折叠模式与动画

| 模式 | 折叠时宽度 | 内容 |
| --- | --- | --- |
| `Icon` | 255 → 48（`COLLAPSED_WIDTH`） | 仍渲染，`icon_collapsed` 下推 |
| `Offcanvas` | 255 → 0，释放布局 | 被裁剪；宽度为 0 时跳过构建内容 |
| `None` | 无动画 | 恒为展开 |

- `align_child_to_end`：Offcanvas 左侧栏收起时内容贴向右缘，右侧栏反之
  —— 移植 gpui-kit 的 `SidebarLayout` 纯函数及其 9 个单测。
- 动画：私有包装 widget `SidebarShell`，持 `from/target/last` 挂在
  `tree::Tag` 的 widget state（同 tabs 的 `StripState`）。`layout` 按当前
  插值宽度给出 `Length::Fixed` 节点并裁剪，内容树始终按目标宽度渲染
  （与 gpui-kit "wrapper 动画、内容不重排" 同策略）。`update` 里遇
  `RedrawRequested` 用 `crate::motion` 的单条 spring 推进，未 settle 请求
  续帧；`reduce_motion()` 时直接到位。
- gpui-kit 的 `finish_hide` 定时器在 iced 侧的等价收益（少构建一层树）由
  "宽度为 0 即跳过内容" 覆盖，实现时确认并注释取舍。

## 主题 token

`Colors` 新增 7 个字段，亮/暗两套。下表为 shadcn 默认值（zinc 系），
**实现时逐一核对 `gpui-kit/crates/component/src/theme` 的实际值，以参考
实现为准**；无该组 token 的主题回退到表中默认值。

| 字段 | 亮色 | 暗色 | 用途 |
| --- | --- | --- | --- |
| `sidebar` | `#f8f8f9` | `#18181b` | 侧栏背景 |
| `sidebar_foreground` | `#404040` | `#d4d4d8` | 侧栏文字 |
| `sidebar_border` | `#e4e4e7` | `#27272a` | 侧栏外边框 |
| `sidebar_accent` | `#e9e9eb` | `#27272a` | hover/active 背景 |
| `sidebar_accent_foreground` | `#18181b` | `#f4f4f5` | hover/active 文字 |
| `sidebar_primary` | `#18181b` | `#f4f4f5` | 侧栏内主色元素 |
| `sidebar_primary_foreground` | `#fafafa` | `#18181b` | 上者的文字 |

纳入现有 WCAG 对比度测试：`sidebar_foreground` on `sidebar`、
`sidebar_accent_foreground` on `sidebar_accent`，两套色板。

## 测试

1. 移植 `SidebarLayout` 的 9 个纯函数单测（bool 兼容、三模式 × 有无像素宽、
   align 方向）与 `SidebarAnimationState` 的 hide 时序用例（按 iced 语义
   裁剪）。
2. 移植 collapsed tooltip 有无的 2 个单测。
3. 新增：disabled 不发消息、`None` 忽略 collapsed、stale active 不 panic。
4. `tests/render.rs` 增加 sidebar 展开 / Icon 折叠 / 暗色三个快照用例
   （`reduce_motion` 下为稳定终态）。

## 集成

- `widgets/mod.rs` 导出（见上）。
- `examples/gallery.rs` 增加 Sidebar 演示区，含三种模式切换
  （对照 `gpui-kit/examples/sidebar`）。
- `README.md` Navigation/Shell 一节加 `Sidebar`；`THEMING.md` token 表补
  colors 新字段。

## 验收

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features dock -- -D warnings
cargo test
cargo test --features dock
```

新单测与三个快照用例全部通过；gallery 中三种折叠模式与动画行为与
gpui-kit 示例一致。
