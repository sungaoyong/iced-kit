# iced-kit 组件使用文档（中文）

本文档面向使用 iced-kit 构建应用的开发者，覆盖组件的构造方式、常用配置，以及本库与
gpui-kit 不同的几个架构约定。组件清单与英文说明见 [README](../README.md)；每个组件
的完整参数见源码 doc 注释，所有示例都能在 `examples/gallery.rs` 里找到可运行的对应物。

## 快速上手

iced-kit 是 [iced](https://iced.rs/) 0.14 的组件库，移植了
[gpui-kit](https://github.com/longbridge/gpui-kit) 的视觉语言（shadcn/ui 风格）。
把应用的主题类型换成 `iced_kit::Theme` 即可全部生效：

```rust
use iced_kit::Theme;

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .theme(App::theme) // 返回 iced_kit::Theme
        .run()
}

fn theme(&self) -> Theme {
    if self.dark { Theme::dark() } else { Theme::light() }
}
```

`prelude` 里带了常用项：

```rust
use iced_kit::prelude::*; // widgets::* + Size + Theme + IconName
```

## 三个核心概念

### 1. 尺寸令牌（Size）

几乎所有控件的尺寸都收在 `Size` 一个枚举里——高度、内边距、图标与文字档位一起变化，
不必逐个对齐像素：

```rust
button("保存").size(Size::Sm)          // Xs / Sm / Md / Lg / Custom(像素)
combobox::<usize, Message>(&options, None, "选择").fill(true)
```

自定义调色板从令牌构造：

```rust
use iced_kit::{Theme, Tokens};

let mut tokens = Tokens::light();
tokens.colors.primary = iced::Color::from_rgb8(0x6d, 0x28, 0xd9);
let theme = Theme::from_tokens(tokens);
```

### 2. 浮层托管模型（`overlay::Layer` + `trigger`）

iced 没有窗口级 z-order，任何控件都无法把弹层画到兄弟节点之上。iced-kit 的约定是
**"组件报告意图，应用托管面板"**：

- 触发器（combobox、date_picker、color_picker 的收起态、菜单按钮……）只发一条消息，
  说明"我被按下了"；
- 应用在 `view` 里把当前打开的面板交给 `overlay::layer(content, open)` 统一装配。

面板要"挂"在触发器旁边，就需要知道触发器在哪。iced 的按钮按压不带坐标，也没有任何
API 能询问一个控件的位置——`trigger` 补上了这一环：它包装任意触发器，按压落在其范围
内的那一刻，**先于**内容消息发布触发器的窗口矩形：

```rust
use iced::widget::{container, stack};
use iced::{Alignment, Length, Padding};
use iced_kit::widgets::overlay::{layer, popover_dismiss_area, trigger, Layer};

enum Message {
    Anchor(iced::Rectangle), // trigger 发布的锚点
    ToggleCombo,             // 触发器自己的意图
    CountryPicked(usize),
    ClosePanel,
}

fn view(&self) -> Element<'_, Message, Theme> {
    // 触发器：报告意图 + 报告位置
    let combo = trigger(
        combobox::<usize, Message>(&self.options, self.picked, "国家")
            .on_toggle(Message::ToggleCombo)
            .fill(true),
        Message::Anchor,
    );

    // 面板：应用托管，锚定在触发器矩形下方 8px，点外面关闭
    let mut open = Layer::new();
    if self.combo_open {
        if let Some(a) = self.panel_anchor {
            let panel = combobox_panel(&self.options, &[self.picked.unwrap()], &self.query,
                                       Message::CountryPicked);
            open = open.dropdown(stack![
                popover_dismiss_area(Message::ClosePanel),
                container(panel)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(Alignment::Start)
                    .align_y(Alignment::Start)
                    .padding(Padding {
                        top: a.y + a.height + 8.0,
                        left: a.x,
                        ..Padding::ZERO
                    }),
            ]);
        }
    }

    overlay::layer(column![combo, /* 页面其余部分 */], open)
}
```

`Layer` 按绘制顺序装配：页面 → 打开的 dropdown/菜单 → drawer → 模态对话框 → toasts。
背板（scrim）点击关闭、动画入场等都是面板自己的事，应用只决定"开不开、开在哪"。

### 3. 离场动画与 `Presence`

iced 无法给"已经不再构建"的东西画动画，所以离场由应用持有的 `Presence` 驱动：
关闭后继续把面板传给 `Layer`，直到 `should_render()` 变假。

```rust
// 关闭时
self.drawer_open = false;
self.drawer_presence.show(false, std::time::Instant::now());

// view 里：开着、或离场还没画完，都继续提供它
if self.drawer_open || self.drawer_presence.should_render() {
    open = open.drawer(Drawer::new("详情", body).presence(&self.drawer_presence));
}
```

> 提示：背板的 dismiss 消息最好只在"确实开着"时生效，否则离场期间的点击会把面板
> 翻回打开。gallery 的 `DrawerDismissed` 就是这么处理的。

---

## 按钮与开关类

### Button / icon_button

```rust
use iced_kit::widgets::{button, icon_button, ButtonVariant};

button("主要操作").primary().on_press(Message::Save)      // primary / secondary /
button("次要").secondary().on_press(Message::Save)        // default / ghost / link /
button("危险").destructive().on_press(Message::Delete)    // destructive / outline …
button("小号").size(Size::Sm).outline(true)
button("加载中").loading(true)                            // 转圈并禁点
button("禁用").on_press_maybe(None)                       // 不设置 on_press 即禁用
icon_button().icon(IconName::X).ghost().on_press(Message::Close) // 图标方钮
```

变体、选中态（环形高亮）、尺寸都来自主题。带菜单的分裂按钮用 `DropdownButton`：
动作半边 + 菜单触发半边，触发只报告按压，菜单由应用经 `Layer` 托管。

### ButtonGroup / Toggle / ToggleGroup

```rust
// 分段按钮：应用持有选中集合
ButtonGroup::new()
    .push(Button::new("日").selected(self.selection.contains(&0)))
    .push(Button::new("周").selected(self.selection.contains(&1)))
    .on_select(Message::GroupSelected)

// 开关类控件
checkbox("通知", self.notifications, Message::NotificationsToggled)
switch("深色", self.dark, Message::ToggleTheme)
radio("免费版", Plan::Free, Some(self.plan), Message::PlanPicked)

// 组合开关：整组状态由应用持有，on_change 回报整组
ToggleGroup::new()
    .push(Toggle::new("粗体").checked(self.bold))
    .push(Toggle::new("斜体").checked(self.italic))
    .segmented(true) // 接缝式外框
    .on_change(Message::TogglesChanged)
```

### slider

```rust
slider(0.0..=100.0, self.volume, Message::VolumeChanged).width(Length::Fixed(320.0))
```

---

## 文本输入

### text_input / password / text_area

```rust
use iced_kit::widgets::{text_input, password, text_area};

text_input::<Message>("邮箱", &self.email)
    .on_input(Message::EmailChanged)
    .on_submit(Message::Submit)
    .clearable(Message::Clear)          // 尾部出现清空按钮
    .invalid(!self.email_valid)         // 错误描边（即使聚焦也保持）

password("密码", &self.password)
    .on_input(Message::PasswordChanged)
    .on_mask_toggle(Message::ToggleVisibility) // 显隐眼睛按钮

// 多行编辑走 text_editor 的 Content 与 Action
text_area("备注", &self.notes).on_edit(Message::NotesEdited)
```

### number_input / otp_input

```rust
number_input("端口", self.port, Message::PortChanged)          // 步进 + 手输
otp_input(&self.otp, Message::OtpChanged)                      // 分格验证码
```

### input_group

把前缀/后缀、标签和错误说明组合成一个整体字段：

```rust
input_group::<Message>()
    .input(text_input::<Message>("example.com", &self.email).on_input(Message::EmailChanged))
    .addon(addon().push(label("https://").size(13)))   // 前缀/后缀 addon
    .width(320.0)
```

---

## 下拉选择：select 与 combobox

两者是**同一控件的两个能力档位**，触发器样式完全一致，按需选用：

### select —— 零成本的简单下拉

菜单由 iced 自己绘制和托管：不需要开合状态、锚点、面板，一行就是一个完整控件。
适合设置面板、表格过滤器这类"只想放个下拉"的场景。不支持搜索与多选。

```rust
use iced_kit::widgets::{select, select_fill};

select(vec![Plan::Free, Plan::Pro, Plan::Team], Some(self.plan), Message::PlanPicked)
    .placeholder("选择套餐")
    .width(160.0)

select_fill(options, selected, on_selected)   // 填满容器，用于表单列
```

### searchable_select —— combobox 的可搜索别名

```rust
// 触发器（面板仍需应用托管，见 combobox）
searchable_select::<usize, Message>(&options)
    .query(&self.query)
    .on_query(Message::Queried)
    .open(self.open)
    .on_toggle(Message::Toggled)
```

### combobox —— 可搜索、可多选、可清空

拆成触发器 + 面板两半；面板经 `Layer` 托管，锚点来自 `trigger`（见核心概念 2）。

```rust
use iced_kit::widgets::{combobox, combobox_panel, ComboBoxOption};

// 选项可以带副行（detail）、可以禁用
let options = vec![
    ComboBoxOption::new(0, "德国").detail("DE"),
    ComboBoxOption::new(1, "加纳"),
    ComboBoxOption::new(2, "日本").disabled(true),
];

// 触发器：可搜索（query 显示在触发器上）、可多选
combobox::<usize, Message>(&options, self.picked.first().copied(), "国家")
    .query(&self.query)
    .on_query(Message::QueryChanged)
    .open(self.combo_open)
    .on_toggle(Message::ToggleCombo)
    .clearable(Message::Clear)
    .multi(true)
    .fill(true)

// 面板：过滤是纯函数，可以按别名、编号等超出标签的字段匹配
combobox_panel(&options, &self.picked, &self.query, Message::CountryPicked)
    .width(260.0)
    .max_height(200.0)
    .on_query(|q| Message::QueryChanged(q))   // 需要面板内搜索框时
```

匹配谓词默认按标签前缀/子串过滤；`match_count(|opt, q| ..)` 可用自定义规则预演过滤。

---

## 日期与颜色

### calendar + date_picker

日期类型是本库自有的 `widgets::date::Date`（无时区的 `(年, 月, 日)`），不引入日期库。
`Calendar` 是纯月历视图（应用持有当前月份）；`date_picker` 是可编辑的 `YYYY-MM-DD`
字段，日历面板同样由应用托管 + `trigger` 锚定。

```rust
use iced_kit::widgets::{calendar, date_picker, date::Date};

// 独立月历：范围高亮、双月视图、自定义周起始日
calendar::<Message>(month, Some(selected))
    .on_select(Message::DatePicked)
    .number_of_months(2)
    .first_day_of_week(iced_kit::widgets::date::Weekday::Sunday)
    .range((start, end))

// 字段：on_input 时可键入，解析交给应用（date::parse）
date_picker::<Message>("选择日期", &self.text)
    .open(self.date_open)
    .on_input(Message::DateText)
    .on_toggle(Message::ToggleDatePicker)
```

### color_picker + color_picker_panel

HSV 空间的取色器：饱和度/明度方块 + 色相条 + 常用色板 + hex 字段。

```rust
use iced_kit::widgets::{color_picker, color_picker_panel, parse_hex};

// 触发器：显示当前颜色的小色块
color_picker::<Message>(self.color)
    .label("主题色")
    .open(self.color_open)
    .on_toggle(Message::ToggleColorPicker)

// 面板：on_change 在点按/拖动时实时回报
color_picker_panel::<Message>(self.color)
    .on_change(Message::ColorChanged)
    .on_hex_input(Message::HexChanged)
    .swatches([palette_a, palette_b, palette_c])
    .width(240.0)
```

---

## 表单布局（form / field）

`Form` 把带标签的字段排成纵向、横向或网格布局，`Field` 描述单个字段：

```rust
use iced_kit::widgets::{form, field};

form::<Message>()
    .label_layout(iced_kit::widgets::FormLabelLayout::Horizontal) // 标签在左
    .label_width(120.0)
    .columns(2)                              // 两列网格
    .children(vec![
        field()
            .label("名称")
            .description("应用对外显示的名字")
            .required(true)
            .push(text_input::<Message>("My app", &self.name).on_input(Message::NameChanged))
            .col_span(2),                   // 占满两列；另有 col_start / col_end
        field()
            .label("邮箱")
            .push(text_input::<Message>("a@b.c", &self.email).on_input(Message::EmailChanged)),
    ])
    .footer(row![button("提交").primary().on_press(Message::Submit)].spacing(8))
```

`visible(false)` 隐藏字段但保留状态；`items_end()` 让控件与标签底对齐（多行控件用）。

---

## 显示组件

```rust
use iced_kit::widgets::{
    card, group_box, divider, vertical_divider, horizontal_separator,
    badge, badge_builder, progress, alert, alert_builder, empty_state, label,
    description_list, link, clipboard_button, rating, avatar, avatar_with_name, avatar_group,
    Description,
};

card(content)                                  // 白底圆角卡片
card(column![heading("标题", Heading::H4), divider(), body].spacing(12))
group_box::<Message>()
    .variant(GroupBoxVariant::Outline)         // Normal / Fill / Outline
    .title(label("分组").size(14))
    .description("带边框的分组面")
    .push(content)

divider()                                      // 水平细分隔线
horizontal_separator().label("或").dashed()    // 带文字的（虚线）分隔

badge("默认", Tone::Neutral)                   // Neutral/Success/Warning/Danger/Primary
badge_builder(Tone::Danger).count(120).max(99) // 99+ 溢出计数；.dot() 圆点徽标

progress(0.4, Tone::Primary)                   // 进度条；ring_progress(...) 为环形

alert("删除成功", Tone::Success)               // 一句话提醒
alert_builder("确认删除？", Tone::Danger)
    .description("此操作不可撤销")
    .banner()                                  // 通栏形态
    .on_close(Message::CloseAlert)

// 空状态：居中的灰调提示（标题 + 描述两段文字）
empty_state("这里还什么都没有", "创建第一个项目开始")

label("机密内容").masked(true)                 // •••• 掩码，带查看按钮
description_list(vec![                         // 键值对描述列表
    Description::new("版本", "0.1.0"),
    Description::new("许可", "MIT OR Apache-2.0"),
])
link("打开文档").href("https://iced.rs")       // 或 .on_press(Message::Open)
clipboard_button("cargo test").label("复制")   // 点击复制，短暂显示"已复制"
rating::<Message>(4).max(5).on_select(|v| Message::Rate(v))
avatar("孙高勇", 32)                           // 姓名取首字母的圆头像
avatar_with_name("孙高勇", "在线")
avatar_group::<Message>()                      // 重叠头像 + 溢出 +N
    .child(avatar("孙高勇", 32))
    .child(avatar("李四", 32))
    .limit(4)
    .ellipsis()
```

树视图（调用方持有展开状态与消息通道）：

```rust
use iced_kit::widgets::{tree, Tree, TreeItem};

let items = vec![TreeItem::new("src", "src").children(vec![
    TreeItem::new("main", "main.rs"),
])];
tree::<Message>(&items, &self.tree_state)
    .on_event(|event| Message::TreeEvent(event)) // 展开/收起/选中等
    .max_height(240.0)
```

---

## 反馈与加载

```rust
use iced_kit::widgets::{spinner, spinner_styled, ring_progress, skeleton, skeleton_list_item,
                        skeleton_table, collapsible, shimmer, shimmer_text, shimmer_block};

spinner(16)                                    // 弧形转圈（u16 直径）
spinner_styled(16, SpinnerStyle::Dots)         // 点阵变体
ring_progress(0.7)                             // 环形进度

skeleton(64.0)                                 // 呼吸占位块
skeleton_list_item(); skeleton_table(3, 4)     // 列表/表格骨架屏（列数, 行数）

shimmer_text("正在生成回答…")                  // 文字扫光
shimmer(element)                               // 任意元素扫光
shimmer_block(240.0, 80.0)
breathe(element)                               // 呼吸明暗（无扫光路径的降级）

// 展开折叠：open 变化时以动画开合，内容是否继续提供由应用决定
collapsible(content, self.open)
```

### Toast

```rust
use iced_kit::widgets::overlay::{Toast, ToastKind, ToastPlacement, Toasts};

// 应用状态里：toasts: Vec<&'static str>，push 即入列，pop 即关闭
let mut toasts = Toasts::new()
    .placement(ToastPlacement::BottomRight)
    .push(Toast::new("已保存", ToastKind::Success).description("就在刚才"))
    .push_dismissible(Toast::new("出错了", ToastKind::Danger), Message::DismissToast);
open = open.toasts(toasts);
```

---

## 数据展示

```rust
use iced_kit::widgets::{list, ListItem, VirtualList, DataTable, Column, Width, SortKey};
use iced_kit::widgets::data_table::{text_cell, number_cell};
use iced_kit::widgets::MarkdownDocument;

// 解析一次、状态持有；渲染时把链接点击映射成消息
let document = MarkdownDocument::parse("# 标题\n正文与 `code`。");
document.into_element().map(|_url| Message::LinkOpened)

// 列表：当前项高亮
list(items, Some(self.selected), |i| Message::ListPicked(i))

// 虚拟列表：可变行高，只渲染可见窗口
VirtualList::new(&self.rows, &self.list_state, |row: &Row, _i| {
    text(row.name).into()
})
.on_scroll(Message::Scrolled)

// 数据表：排序 + 虚拟化 + 固定列 + 分组表头 + 骨架加载
let columns = vec![
    Column::<Row, Message>::new("名称", |row: &Row, _index| text_cell(row.name.clone()))
        .width(Width::Fill)
        .sortable(|row| SortKey::text(row.name.clone())),
    Column::<Row, Message>::new("大小", |row: &Row, _index| number_cell(format_bytes(row.size)))
        .width(Width::Fixed(120.0))
        .align_x(Alignment::End)
        .sortable(|row| SortKey::number(row.size as f64)),
];
DataTable::new(&self.rows, &self.table_state, columns)
    .row_height(30.0)
    .max_height(240.0)
    .on_sort(Message::Sorted)
```

Markdown 渲染：`MarkdownDocument::parse` 解析一次、状态持有，`into_element()` 渲染并把
链接点击以消息回报。

### 图表

七种图共用主题色板；折线/柱状图以横轴标签 + 序列描述数据：

```rust
use iced_kit::widgets::{LineChart, LineSeries, BarChart, BarAlignment, PieChart, PieSlice,
                        RadarChart, CandlestickChart, SankeyChart};

// 折线图：横轴标签 + 序列（值是 Vec<f64>）
LineChart::new(
    (0..24).map(|h| format!("{h}时")).collect(),
    vec![
        LineSeries::new("读", samples.clone()).tone(Tone::Primary),
        LineSeries::new("写", writes.clone()).color(my_color),
    ],
)
.height(170.0)

BarChart::new(series).alignment(BarAlignment::Stacked) // Grouped / Stacked，另有横向
PieChart::new(vec![PieSlice::new("A", 40.0)]).donut(true) // 环形图
```

---

## 导航

```rust
use iced_kit::widgets::{tabs, Tab, accordion, AccordionSection, breadcrumb, Crumb,
                        stepper, Step, StepLayout, pagination, carousel, CarouselState,
                        app_menu_bar, MenuTitle, MenuItem, sidebar, SidebarHeader,
                        SidebarGroup, SidebarFooter, SidebarMenuItem, SidebarCollapsible};

// 页签：underline / tab / outline / pill / segmented 五种变体
tabs(vec![Tab::new("常规"), Tab::new("高级").enabled(false)], self.tab, Message::TabSelected)
    .variant(TabVariant::Pill)

// 手风琴：多开、带边框、每节可带图标；内容按节惰性构造（第四个闭包）
accordion(
    vec![
        AccordionSection::new("常规").subtitle("基础选项"),
        AccordionSection::new("高级").subtitle("专家选项"),
        AccordionSection::new("危险区"),
    ],
    self.open_index,          // Option<usize>；None 即全部收起
    Message::AccordionToggled,
    |index| label(format!("第 {index} 节的内容。")).into(),
)

// 面包屑与步骤条
breadcrumb(vec![Crumb::link("首页", Message::Noop), Crumb::new("iced-kit")])
stepper(vec![Step::new("购物车"), Step::new("地址")], 1)
    .layout(StepLayout::Horizontal)
    .on_select(|i| Message::StepPicked(i))

// 分页
pagination(self.page, 12, Message::PageSelected)

// 轮播：状态由应用持有；支持循环、圆点指示、方向键与拖拽吸附
carousel(&self.carousel_state, slides, Message::CarouselSelected)
    .looping(true)

// 应用菜单栏：只排标题条，点击回报打开的标题索引；下拉内容由应用经 Dropdown 托管
app_menu_bar(
    vec![MenuTitle::new("文件"), MenuTitle::new("编辑")],
    self.open_menu,           // Option<usize>
    Message::MenuOpened,
)
```

### 侧边栏（Sidebar）

```rust
sidebar()
    .collapsible(SidebarCollapsible::Icon) // Icon / Offcanvas / None
    .collapsed(self.collapsed)
    .width(self.width)
    .min_width(200.0)
    .on_resize(Message::Resized)           // 可拖拽调宽
    .header(SidebarHeader::new().child(logo_row))
    .child(SidebarGroup::new("应用").child(
        column![
            SidebarMenuItem::new("项目", Message::Projects)
                .icon(IconName::Folder)
                .active(self.picked == "项目"),
            SidebarMenuItem::new("设置", Message::Settings)
                .children(submenu)   // 带子项即成子菜单（出现折叠箭头）
                .open(true)
                .on_toggle(Message::SubmenuToggled),
        ]
        .spacing(2),
    ))
    .footer(SidebarFooter::new().child(user_row))
```

折叠为图标档时标签消失、保留 tooltip；子菜单在窄档下自动收纳。

---

## 浮层

所有浮层的装配入口都是 `overlay::layer(content, open)`（见核心概念 2）。

### Modal / Dialog / AlertDialog

```rust
use iced_kit::widgets::overlay::{Modal, AlertDialog, AlertTone, drawer_header, drawer_actions};

// 模态：标题 + 说明 + 动作；背板点击关闭，可禁用
open = open.modal(
    Modal::new("删除项目", label("该项目及其所有数据将被删除。"), Message::CloseModal)
        .description("此操作不可撤销。")
        .cancel("取消", Message::CloseModal)
        .destructive("删除", Message::ConfirmDelete)
        .draggable(true),   // 卡片可拖走，方便看被盖住的内容
);

// 警告框：中断性语气，图标随 tone，按钮居中
open = open.modal(
    AlertDialog::new()
        .tone(AlertTone::Danger)
        .title("清空回收站？")
        .description("已删除的项目将永久消失。")
        .ok_text("清空")              // 确认键文案
        .confirm()                    // 确认/取消成对出现
        .on_confirm(Message::Empty)
        .on_cancel(Message::Close)
        .on_dismiss(Message::Close),
);
```

对话框也可以用 `DialogHeader` / `dialog_title` / `dialog_description` / `DialogContent` /
`DialogFooter` 自行拼装，交给 `Layer::modal`。

### Drawer / sheet / HoverCard

```rust
use iced_kit::widgets::overlay::{drawer_header, Drawer, DrawerSide, sheet};

open = open.drawer(
    Drawer::new("", column![paragraph("…"), text_input::<Message>("名称", "")].spacing(12))
        // 头部自带关闭按钮：drawer 上的按压不会漏给背板，没有可见的关闭
        // 途径时只能点外面退出
        .header(drawer_header("详情", Message::DrawerDismissed))
        .side(DrawerSide::Right)      // Left / Right / Bottom
        .presence(&self.drawer_presence)
        .on_dismiss(Message::DrawerDismissed),
);

// sheet：从窗口边缘滑入、止步于标题栏之下的轻量面板
sheet("通知", body).side(DrawerSide::Bottom)
```

关闭途径有两处：点 ✕（或应用放进 header/footer 的任何按钮），以及点 drawer 之外的背板。
面板自身的空白处不会误关——那是有意的行为，所以**一定要提供 header 里的关闭钮**。

### Dropdown / ContextMenu / Popover

```rust
use iced_kit::widgets::overlay::{Dropdown, ContextMenu, Popover, PopoverPlacement};

// 菜单：锚点常来自 trigger 或 ContextMenu 的右键位置
open = open.dropdown(
    Dropdown::new(vec![
        MenuItem::new("复制", Message::Copy).shortcut("Ctrl+C").icon("⧉"),
        MenuItem::new("删除", Message::Delete).destructive(true),
        MenuItem::new("归档", Message::Archive).enabled(false),
    ])
    .anchor(point.x, point.y)
    .align(DropdownAlign::End),   // 贴右缘时向内展开
);

// 右键菜单
open = open.dropdown(ContextMenu::new(items, (x, y)));

// 气泡卡片：任意内容，四向放置
open = open.dropdown(
    Popover::new(column![heading("快捷设置", Heading::H4), body].spacing(12), (x, y))
        .placement(PopoverPlacement::BottomStart)
        .width(280.0),
);
```

`popover_dismiss_area(message)` 是点外关闭用的全屏透明捕捉层，垫在面板下方即可。

### tooltip 与 kbd

```rust
use iced_kit::widgets::{tooltip, tooltip_with_shortcut, kbd, shortcut, TooltipPosition};

tooltip(element, "保存并关闭".to_owned())
tooltip_with_shortcut(button("保存").primary(), "保存", "Ctrl+S") // 快捷键提示
```

---

## 布局外壳

```rust
use iced_kit::widgets::{TitleBar, Resizable, SplitAxis};
use iced::widget::pane_grid;

// 自绘标题栏（窗口控制按钮的点击会回报，由应用决定调用窗口 API）
TitleBar::new("我的应用")
    .on_minimize(Message::Minimize)
    .on_maximize(Message::Maximize)
    .on_close(Message::Close)

// 可拖拽分割：分割状态是 pane_grid::State，由应用持有
let (splits, _) = Resizable::<Message>::split_state(3, SplitAxis::Horizontal, 0.25);

Resizable::new(&splits)
    .min_size(60.0)
    .on_resize(|_event| Message::Resized)
    .pane(|_pane, index: &usize| pane_body(index).into())
```

`Resizable` 需要 `dock` feature；完整的多页签停靠布局（拖拽页签、嵌套分割、边缘停靠、
面板最大化）也在该 feature 之后。

### 设置面板（Settings）

页面与分组经 builder 传入，选中页、搜索词等面板状态在 `SettingsState` 里（应用持有，
事件回报后经 `apply` 记账）：

```rust
use iced_kit::setting::{Settings, SettingsState, SettingPage, SettingGroup, SettingItem,
                        SettingField};

let panel = Settings::<Message>::new(&self.settings) // self.settings: SettingsState
    .sidebar_width(self.width)      // 侧边栏可拖宽
    .on_sidebar_resize(Message::SidebarResized)
    .on_event(Message::Settings)    // 搜索、选页、滚动请求都以事件回报
    .on_reset(Message::SettingsReset)
    .pages(vec![
        SettingPage::new("外观")
            .icon(IconName::Palette)
            .description("界面如何绘制。")
            .group(
                SettingGroup::new()
                    .title("主题")
                    .description("调色板。")
                    .items(vec![
                        SettingItem::new("深色模式")
                            .description("使用深色调色板。")
                            .keywords(["night", "dark"])      // 搜索命中词
                            .field(SettingField::switch(self.dark, Message::DarkMode)),
                        SettingItem::new("语言").field(SettingField::select(
                            vec!["简体中文".to_owned(), "English".to_owned()],
                            Some(self.locale.clone()),
                            Message::Locale,
                        )),
                        SettingItem::new("字号").field(SettingField::number(
                            self.font_size,
                            12.0..=20.0,
                            Message::FontSize,
                        )),
                        SettingItem::new("用户名").field(SettingField::text(
                            self.name.clone(),
                            Message::Name,
                        )),
                    ]),
            ),
        SettingPage::new("通用").icon(IconName::Settings2).group(/* … */),
    ]);
```

侧边栏可搜索（按标题、描述与关键词命中）；选择分组后面板会以 Task 形式请求滚动到对应
位置（见 `Settings::scroll_to_group`）。字段种类见 `SettingField`：`switch`、`select`、
`number`、`text` 等。

---

## 聊天组件（chat）

面向 IM 界面的一族组件，全部由调用方持有状态：

```rust
use iced_kit::widgets::{avatar, muted_text};
use iced_kit::widgets::chat::{message, bubble, marker, attachment, message_scroller,
                             BubbleVariant, BubbleReactions, MessageAlignment,
                             MessageAvatar, MessageHeader, MessageScrollerState,
                             AttachmentContent, AttachmentTitle, AttachmentDescription,
                             AttachmentStatus};

// 气泡：filled / secondary / muted / tinted / outline / destructive / ghost
bubble("最近怎么样？").with_variant(BubbleVariant::Tinted).alignment(MessageAlignment::End)

// 反应：骑在气泡边缘的胶囊；带按钮的反应可点击
let reactions = BubbleReactions::new().child(muted_text("👍 2"));

// 消息行：头像（或序号槽）+ 头部 + 内容面 + 底部，整体可对齐到任一侧
message(bubble("你好"))
    .avatar(avatar::<Message>("孙高勇", 32))
    .header("孙高勇")
    .footer("12:30")
    .alignment(MessageAlignment::End)

// 标记：分隔线、系统提示；加载态用转圈或扫光
marker::<Message>("— 今天 —").with_variant(MarkerVariant::Border)
marker::<Message>("正在输入…").loading(true)

// 附件：五种上传状态（Pending 虚线卡、Failed 染错误色），尺寸五档
attachment::<Message>("设计稿.png")
    .status(AttachmentStatus::Uploading(0.6))
    .content(
        AttachmentContent::new()
            .title(AttachmentTitle::new("设计稿.png"))
            .description(AttachmentDescription::new("2.4 MB")),
    )

// 滚动区：虚拟化、跟随尾部、底部渐隐 + "跳到最新"
message_scroller(&self.lines, &self.scroller_state, |line, _i| {
    message(bubble(line)).into()
})
.on_jump_to_bottom(Message::JumpToLatest)
```

---

## 与 gpui-kit 的已知差异

- **浮层托管**：gpui-kit 由全局 `Root` 统一管理对话框与通知；iced-kit 有意让应用经
  `overlay::Layer` 显式组装（无窗口级 z-order 的唯一可行解），`trigger` 负责补齐锚点。
- **代码高亮**：未移植 tree-sitter/highlighter 相关能力。
- **无障碍与 i18n**：iced 侧仅有部分等价物，暂未覆盖。
