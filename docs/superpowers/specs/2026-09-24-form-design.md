# Form 组件设计（对齐 gpui-kit）

日期：2026-09-24
状态：设计已确认，待实现

## 目标

把 `gpui-kit/crates/component/src/form`（mod/field/form 三个文件）对齐到
iced-kit：`Form` 是装载多个 `Field` 的布局容器，负责标签方向、多列网格、
尺寸档位和 footer；`Field` 负责单个字段的标签、说明文字、必填标记与网格
占位。API 形态与参考一一对应，行为差异仅来自 iced 的无状态约定。

参考实现是纯布局容器：不持有控件状态、不做校验、不管理提交。校验型表单
状态机不在本次范围内（YAGNI，参考也没有）。

参考实现的行为清单：

- `Form`：`vertical()`/`horizontal()` 设标签在上/在左（默认在上），
  `columns(n)` 网格列数（默认 1），`label_width`（默认 140px，仅水平布局
  生效），`label_text_size`，尺寸档位决定间距，`footer`（跨全部列、尾对齐），
  `child`/`children` 添加字段。
- `Field`：`label`、`description`（弱色小字）、`required`（danger 色 `*`）、
  `visible`、`label_indent`（默认 true，仅水平布局生效）、条目对齐
  （start/end/center）、`col_span`/`col_start`/`col_end` 网格占位。

## 范围外

- 校验、错误消息、提交状态（参考实现没有，另立主题）。
- 公开的独立 `grid` 布局组件（只有 form 用，`pub(crate)` 即可）。
- `FieldBuilder::View` 变体：iced 没有 view 句柄，一切子内容都是
  `Element`（下推见"与参考差异"）。

## 模块结构

新增 `src/widgets/form/` 目录：

- `mod.rs` — `Form`、`Field`、`FormLabelLayout`、`FieldLabel` 与构造函数
  `form()`、`field()`。
- `grid.rs` — 自写网格布局 widget（`pub(crate)`），行优先自动流 + 等宽列。

在 `src/widgets/mod.rs` 导出：

```rust
pub use form::{field, form, Field, FieldLabel, Form, FormLabelLayout};
```

`form()` 与 `Form::new()` 等价，遵循本仓库"模块 + 同名构造函数"的约定；
`field()` 同理。命名核查：`widgets::label` 已被 input 模块的文本助手占用，
`field`/`form` 无冲突。

## 类型与 builder

```rust
pub enum FormLabelLayout { Vertical, Horizontal } // 标签在上 / 在左；默认 Vertical

pub enum FieldLabel<'a, Message> { Text(String), Element(Element<'a, Message, Theme>) }
// From<&str>/From<String> → Text；From<Element> → Element。
// 对应参考的 FieldBuilder（String | Element | View），去掉 View 变体。

pub struct Form<'a, Message> { /* ... */ }
impl<'a, Message: 'a> Form<'a, Message> {
    fn new() -> Self;
    fn vertical() -> Self;                              // = new()
    fn horizontal() -> Self;
    fn layout(self, FormLabelLayout) -> Self;           // label_layout 的别名（参考同名）
    fn label_layout(self, FormLabelLayout) -> Self;
    fn label_width(self, f32) -> Self;                  // 默认 140.0，仅水平布局生效
    fn label_text_size(self, f32) -> Self;              // 标签字号 px，默认随主题 sm
    fn columns(self, usize) -> Self;                    // 默认 1
    fn size(self, Size) -> Self;                        // 档位 → 间距（见视觉规格）
    fn child(self, Field<'a, Message>) -> Self;
    fn children(self, impl IntoIterator<Item = Field<'a, Message>>) -> Self;
    fn footer(self, impl Into<Element<'a, Message, Theme>>) -> Self;
}

pub struct Field<'a, Message> { /* ... */ }
impl<'a, Message: 'a> Field<'a, Message> {
    fn new() -> Self;
    fn label(self, impl Into<FieldLabel<'a, Message>>) -> Self;
    fn description(self, impl Into<FieldLabel<'a, Message>>) -> Self;
    fn required(self, bool) -> Self;                    // 默认 false
    fn visible(self, bool) -> Self;                     // 默认 true
    fn label_indent(self, bool) -> Self;                // 默认 true，仅水平布局生效
    fn items_start(self) -> Self;                       // 标签+控件行的对齐，默认 Start
    fn items_end(self) -> Self;
    fn items_center(self) -> Self;
    fn col_span(self, u16) -> Self;                     // 默认 1
    fn col_start(self, u16) -> Self;                    // 1 起始，同 CSS 语义
    fn col_end(self, u16) -> Self;
    fn push(self, impl Into<Element<'a, Message, Theme>>) -> Self;  // 控件本体
}
```

不需要参考的 `label_fn`/`description_fn`：iced 无状态渲染，view 时元素已
构建好，`label()` 接受 `Element` 已覆盖两者的全部用途（与 sidebar 移植时
同一结论）。`push` 沿用 `GroupBox` 的惯例名；`Element` 不是 `Clone`，
builder 不可 `Copy`/`Debug`，同 `GroupBox` 文档注释的说明。

## 网格算法（`grid.rs`）

实现 `iced::advanced::Widget` 的行优先自动流网格，等宽列，总宽 `Fill`：

1. 把每个格子解析为 `(col_start: Option<u16>, col_end: Option<u16>,
   col_span: u16)`。显式定位的格子先放置；`col_start` 缺省时自动流从上一
   格之后找位，`col_end` 缺省时 `col_end = col_start + col_span - 1`；
   `col_start` 缺省而 `col_end` 给出时，从自动流位置起、终点不超过
   `col_end`。
2. 自动流按序填剩余格子：从当前行的当前列起找能容纳 `col_span` 的连续空
   位，找不到则下移一行（CSS sparse 模式；不回头填洞）。
3. 列宽：`可用宽 = 容器宽 - 列间距×(columns-1)`，每列 `可用宽/columns`；
   `col_span` 格子的宽 = 所跨列宽之和 + 所跨内部列间距。参考的列间距 =
   行间距 × 3，同此。
4. 行高：该行所有格子的最大高度；格子高度交给内容（网格自身不设高）。
5. 放置算法提为纯函数（输入：格子数与每格的 span/start/end、列数；输出：
   每格 `(row, col_start, col_span)`），单测直接测它，不经过渲染。

**footer**：`Form` 把 footer 包装成 `col_span = columns` 的格子追加在网格
尾部，格子内容右对齐（容器 `align_x` End）。几何与参考（`col_span_full`
+ `justify_end` 的网格子项）一致，grid 无需特殊分支。

## 视觉规格（照抄参考数值）

间距由 `Form::size` 决定，参考值映射到本仓库的 `Size`：

| Size（本仓库） | 表单行间距 | 列间距 | 字段内部间距 |
| --- | --- | --- | --- |
| `Xs` / `Sm` | 6 | 18 | 4 |
| `Md`（默认） | 8 | 24 | 4 |
| `Lg` | 12 | 36 | 8 |
| `Custom(v)` | `v` | `3v` | 同 `Md` |

参考的 `Size::XSmall|Small → 6 / Large → 12 / 其余 → 8`，本仓库档位一一
对应；`Custom` 取其像素值直接作行间距。

`Field` 的渲染结构（自上而下）：

- 外层列，间距 = 字段内部间距 / 2（`Md` 档为 2，`Lg` 档为 4）。
- **标签 + 控件行**：垂直布局为列、水平布局为行，间距 = 水平布局 ? 字段
  内部间距 : 字段内部间距 / 2；条目对齐（`items_*`）作用于该行。标签列宽
  = `label_width`（仅水平布局），`flex_shrink` 关闭。
  - 标签文本：`Size::Sm.text()` + `iced::font::Weight::Medium`，内容与
    必填星号间距 4；`required` 时标签后跟 danger 色 `*`（`theme.colors().danger`）。
  - 控件容器：占满剩余宽度（水平布局）/ 整行（垂直布局）。
- **说明文字行**：水平布局且 `label_indent` 且有标签时，先放一个
  `label_width` 宽的空占位对齐控件列，然后是 description 文本：
  `Size::Xs.text()` + `muted_foreground`。

`visible(false)` 的字段在进入网格前被 Form 过滤（参考中 `visible` 只存
不用，属死 API；按意图实现，见"与参考差异"）。

## 与参考的差异（均为 iced 约束或参考自身问题）

1. `FieldBuilder::View` 不移植：iced 没有 view 句柄，`FieldLabel` 只有
   Text / Element 两个变体。
2. `label_fn`/`description_fn` 合并入 `label`/`description`（接受
   `Element`）。
3. `label_text_size` 从 gpui 的 Rems 改为 iced 的像素 f32。
4. gpui 的 `Axis` 换成自有 `FormLabelLayout`（iced 无对应类型；参考已把
   方法改名为 `label_layout`，语义即"标签方向"）。
5. `visible` 在参考中只存不用（死代码）；按意图实现为 Form 过滤不可见
   字段，并在代码注释里说明。
6. `Styled` 链不移植：form 不需要调用方逐项调样式（与 sidebar 同一取舍）。

## 测试

1. **单元测试**（`form/mod.rs` 内）：镜像参考 `test_form_builder` 的默认值
   断言（`new`/`default`/`vertical` 同布局，`columns`/`footer`/字段数）。
2. **网格放置纯函数单测**（`form/grid.rs` 内）：单列顺序排列；`columns(2)`
   自动流两两并排；`col_span` 跨列并触发下一格换行；`col_start` 强制定位；
   `col_end` 收窄可用位置；显式定位与自动流混合时 sparse 不回头。
3. **快照**（`tests/render.rs`）：垂直单列、水平单列（带 description 与
   required）、双列 + footer 三张。
4. **几何测试**（仿 `tests/dialog_centring.rs` 的 PNG 回读测量）：测试
   字段给纯色背景，在像素里找各字段包围盒，断言——双列时字段 0/1 同行且
   字段 2 在下一行行首；垂直布局标签在控件上方、水平布局标签在控件左侧；
   footer 右缘与表单右缘对齐、跨满列宽。对应参考的三个 `debug_bounds`
   测试（列独立于标签方向、footer 跨列尾对齐、footer 缺省不影响几何）。

## 集成

- `widgets/mod.rs` 导出（见上）。
- `examples/gallery.rs` 增加 Form 演示区（对照 gpui-kit 的 form story：
  姓名/邮箱/简介 + required + description + 双列 + footer 按钮）。
- `README.md` Form 一节补 `form`/`field`/`Form`/`Field`。

## 验收

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

新单测、三张快照、几何测试全部通过；gallery 中垂直/水平/双列三种形态与
gpui-kit 的 form story 一致。
