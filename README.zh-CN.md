# iced-kit

基于 [iced](https://iced.rs/) 0.14 的 shadcn/ui 风格组件库与设计系统，移植自 [gpui-kit](https://github.com/longbridge/gpui-kit) 的视觉语言。

## 预览

| 浅色主题 | 深色主题 |
|:---:|:---:|
| ![按钮变体](tests/snapshots/button_variants-wgpu.png) | ![深色模式](tests/snapshots/dark_mode-wgpu.png) |

## 核心理念

iced 0.14 的组件通过泛型 `Theme` 类型进行主题化。iced-kit 自带完整的语义 Token 体系，并为上游组件实现了 `Catalog` trait —— 因此无论是 iced 原生组件还是 iced-kit 组件，都能保持一致的视觉风格。

```rust
use iced_kit::Theme;

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .theme(App::theme)   // 返回 iced_kit::Theme
        .run()
}
```

## 设计 Token

| 分组 | 包含 |
| --- | --- |
| 颜色 | `background`, `foreground`, `surface`, `primary`, `secondary`, `muted`, `accent`, `destructive`, `warning`, `success`, `info`, `link`, `border`, `input`, `ring`, `selection` 及对应的 `*_foreground` |
| 圆角 | `none`, `sm`, `md`, `lg`, `xl`, `full` |
| 间距 | `xxs` … `xxl` |
| 排版 | `sans`, `mono`, 及 `xs`/`sm`/`md`/`lg`/`xl` 字号 |
| 尺寸 | `Xs`, `Sm`, `Md`, `Lg`, `Custom(px)` — 控制高度、内边距、图标和文字步进 |
| 动效 | `instant`, `fast`, `normal`, `slow`，及 `enter`/`exit`/`move` 曲线 |

## 组件一览

> 本项目提供 **100+ 个 UI 组件**，覆盖表单、显示、反馈、数据、图表、聊天、导航、浮层、Shell、设置、动效等全部场景。

| 分类 | 数量 | 关键组件 |
|------|:---:|----------|
| 表单控件 | 22 | Button, TextInput, ComboBox, DatePicker, ColorPicker |
| 表单布局 | 2 | Form, Field |
| 显示组件 | 18 | Card, Badge, Alert, Tree, Rating |
| 反馈组件 | 7 | Spinner, Skeleton, Shimmer, Collapsible |
| 数据展示 | 5 | VirtualList, DataTable, Markdown |
| 图表 | 7 | LineChart, BarChart, PieChart, CandlestickChart, SankeyChart |
| 聊天组件 | 6 | Bubble, Message, Attachment, MessageScroller |
| 导航组件 | 8 | Tabs, Accordion, Sidebar, Carousel, Stepper |
| 浮层与对话框 | 14 | Modal, Drawer, Toast, Dropdown, Popover, Tooltip |
| Docking 停靠 | 3 | Dock, Resizable, TitleBar |
| 设置面板 | 5 | Settings, SettingPage, SettingGroup, SettingItem, SettingField |
| 排版与头像 | 8 | Heading, Code, Avatar, Kbd |
| 图标 | 1000+ | Lucide 图标字体全套 |
| 动效 | — | Presence + 全组件内置动画 |

### 表单控件（22 个）

按钮（11 种变体）、图标按钮、按钮组、下拉按钮、切换按钮（Toggle）、切换组（ToggleGroup）、文本输入、密码输入、多行文本、输入组、选择器、可搜索选择器、组合框（多选+过滤面板）、复选框、单选框、开关（Switch）、滑块、数字输入、OTP 输入、日历、日期选择器、颜色选择器。

![表单控件](tests/snapshots/form_controls-wgpu.png)

![输入组](tests/snapshots/input_groups-wgpu.png)

### 表单布局（2 个）

`form`/`field` 将带标签的控件排列为表单：支持纵向或横向标签放置、`columns` 网格布局与 `col_span`/`col_start`/`col_end` 定位、`required` 标记、muted 描述行，以及底部操作区。

![纵向表单](tests/snapshots/form_vertical-wgpu.png)

![多列表单](tests/snapshots/form_columns-wgpu.png)

### 显示组件（18 个）

卡片、分组框、分割线、垂直分割线、水平分隔线（带标签/虚线）、Badge（标签/圆点/计数/图标形式）、进度条、Alert（横幅形式+关闭按钮）、空状态、标签、Tag、头像组（重叠+溢出标记）、Label Builder（掩码值+搜索高亮）、描述列表、链接、剪贴板按钮、评分、树形控件。

![显示组件](tests/snapshots/display-wgpu.png)

### 反馈组件（7 个）

Spinner（弧形和圆点）、环形进度、骨架屏、骨架列表项、骨架表格、折叠面板、Shimmer（文本/区块/任意元素，可配置扫过方向）。

![反馈组件](tests/snapshots/feedback-wgpu.png)

### 数据展示（5 个）

列表、ListItem、VirtualList（可变高度虚拟化）、DataTable（可排序、虚拟化、分组标题、固定列、骨架加载状态）、Markdown。

![虚拟列表](tests/snapshots/virtual_list-wgpu.png)

![数据表格](tests/snapshots/data_table-wgpu.png)

### 图表（7 种）

折线图、面积图（叠加/堆叠）、柱状图（分组/堆叠，四种方向）、饼图（饼/环）、雷达图、K 线图、桑基图。所有图表共享悬停十字线、数据提示、刻度映射和主题色彩体系。

![折线/面积/柱状图](tests/snapshots/line_chart-wgpu.png)

![饼图](tests/snapshots/pie_chart-wgpu.png)

![K线图](tests/snapshots/candlestick_chart-wgpu.png)

![桑基图](tests/snapshots/sankey_chart-wgpu.png)

### 聊天组件（6 个）

气泡（7 种表面样式+反应胶囊）、消息（头像/编号+头部+内容+底部）、消息组、标记（普通/分隔/边框，加载动画）、附件（5 种上传状态+预览+操作行）、消息滚动器（虚拟化、跟随尾部、底部渐隐、跳转最新）。

![聊天气泡](tests/snapshots/chat_bubbles-wgpu.png)

![消息排列](tests/snapshots/chat_message_rail-wgpu.png)

![附件](tests/snapshots/chat_attachments-wgpu.png)

### 导航组件（8 个）

标签页（下划线/标签/轮廓/胶囊/分段五种风格，支持尺寸和图标插槽）、手风琴（多项展开/带边框/尺寸/图标）、面包屑、步骤条（水平/垂直，标记完成状态）、应用菜单栏、分页、轮播（水平/垂直，前后控制/圆点指示/循环/键盘/拖拽吸附）、侧边栏（可折叠面板，带头部/分组菜单/底部；三种折叠模式，动画宽度过渡，可拖拽调整宽度，子菜单，Badge，折叠态工具提示）。

![导航](tests/snapshots/navigation-wgpu.png)

![侧边栏](tests/snapshots/sidebar-wgpu.png)

### 浮层与对话框（14 个）

Modal、Dialog、AlertDialog、Drawer、Sheet（标题栏下方停驻）、Toast/Toasts、Dropdown/MenuItem、ContextMenu、Popover、HoverCard（悬停打开）、Tooltip（可指定快捷键）。支持 `trigger` 锚点机制，让面板精确定位到触发器位置。

![模态框](tests/snapshots/modal-wgpu.png)

![抽屉](tests/snapshots/drawer-wgpu.png)

![消息提示](tests/snapshots/toasts-wgpu.png)

![下拉菜单](tests/snapshots/dropdown-wgpu.png)

### Docking 停靠布局（`dock` feature，3 个）

完整的可停靠布局系统：可拖拽标签、嵌套分割、拖放目标、可折叠边缘 Dock 和面板缩放。

![Dock 布局](tests/snapshots/dock-wgpu.png)

### 设置面板（5 个）

搜索侧边栏 + 页面/分组/条目/字段四级结构，支持 Switch、Checkbox、文本、数字步进、下拉选择和自定义字段。面板可拖拽调整大小。

![设置面板](tests/snapshots/settings_panel-wgpu.png)

### 排版与头像（8 个）

标题、段落、辅助文本、代码、键盘按键标记、快捷键组合、头像、带头像的名称。

![排版](tests/snapshots/typography-wgpu.png)

### 图标（Lucide 全套 1000+ 图标）

集成 [Lucide](https://lucide.dev) 全套图标字体，通过 `IconName` 枚举直接使用，无需手动管理 SVG 文件。

![图标](tests/snapshots/named_icons-wgpu.png)

### 动效（Presence + 全组件内置动画）

所有表面都有入场/出场动画：Drawer 从边缘滑入、Dialog 升起、Dropdown 下落、Toast 从角落进入。标签指示器在标签间滑动、折叠面板展开动画、进度条缓动、骨架屏呼吸效果。

## 快速上手

```toml
# Cargo.toml
[dependencies]
iced-kit = "0.1"

# 启用 Dock 功能
iced-kit = { version = "0.1", features = ["dock"] }
```

```rust
use iced_kit::prelude::*;
use iced_kit::Theme;

fn view(&self) -> Element<'_, Message, Theme> {
    column![
        heading("Hello iced-kit"),
        button("Save").primary().on_press(Message::Save),
        badge("Active", Tone::Success),
        alert("Saved", "Your changes were written.", Tone::Success),
    ]
    .spacing(16)
    .padding(24)
    .into()
}
```

## 运行示例

```sh
# 组件画廊
cargo run --example gallery

# 深色模式启动
GALLERY_DARK=1 cargo run --example gallery

# Dock 布局示例
cargo run --example dock --features dock
```

## 开发

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features dock -- -D warnings
cargo test
cargo test --features dock
```

## 文档

- [THEMING.md](THEMING.md) — 自定义 Token、组件样式覆盖
- [docs/COMPONENTS.zh-CN.md](docs/COMPONENTS.zh-CN.md) — 中文组件使用文档

## 致谢

设计语言、Token 模型和组件命名来自 [gpui-kit](https://github.com/longbridge/gpui-kit)（Apache-2.0）和 [shadcn/ui](https://ui.shadcn.com/)。实现参考了 [iced-astraui](https://github.com/AstraBrew-Labs/iced-astraui) 和 [iced-shadcn](https://github.com/FerrisMind/shadcn-rs)（均为 MIT）。

## 许可证

MIT OR Apache-2.0
