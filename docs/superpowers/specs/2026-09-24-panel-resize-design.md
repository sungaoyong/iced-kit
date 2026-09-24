# 面板拖拽调宽设计(Sidebar 与 Settings 侧栏)

日期:2026-09-24
状态:已实现

## 目标

给 `Sidebar` 与 `Settings` 面板的侧栏添加用户拖拽调宽能力,交互对齐
`Resizable`(dock 示例的 split 拖拽)。

## 参考实现的交互(iced `pane_grid`)

- 分隔线空闲不可见;悬停显示 `ring` 色 2px 线,拖拽中显示 primary 色 2px 线
  (`resizable.rs::splitter_style`)。
- 抓取区 6px(`grid.on_resize(6.0, ..)`),光标为左右调整形。
- 拖拽即时跟手、无动画,拖拽结束上报 `ResizeEvent`。

## 方案比较

1. **包一层 `pane_grid`**——重:引入 Pane/State/Split 概念与比例语义,与
   Sidebar 自身的折叠动画(宽度是像素、由 spring 驱动)冲突。否。
2. **拖拽内建于 `SidebarShell` + `Settings` 用一个共享的 `ResizeEdge` 包装
   widget(选用)**——shell 本就持有 spring 状态与 bounds,拖拽时直接
   `set()` 宽度即可精确跟手;`Settings` 侧栏是普通固定宽列,一个"贴在
   子元素内容侧边缘的拖拽条"包装 widget 即可,不引入动画。
3. **两个组件各自实现拖拽**——交互常量与数学重复。否;数学抽成共享纯函数。

## 设计

- 共享原语 `src/widgets/resize_edge.rs`(pub(crate)):
  - `GRAB_WIDTH = 6.0`、`LINE_WIDTH = 2.0`、`MIN_CONTENT_AREA = 240.0`
    (内容区地板,防止把内容拖没,对应 pane_grid 的 min_size 语义)。
  - 纯函数 `dragged_width(press_width, press_x, cursor_x, anchored_left,
    min, max) -> f32`:左侧停靠面板向右拖变宽、右侧停靠相反,结果夹在
    `[min, max(max, min)]`。两个组件共用,单测覆盖。
  - `ResizeEdge` 包装 widget:子元素是固定宽面板,抓取条覆盖其内容侧
    最后 6px;空闲不画(面板自带 hairline 或现有 rule 提供线条),悬停/
    拖拽画 2px 高亮线(颜色与 pane_grid splitter 一致);拖拽按下不向子
    组件转发该次按下,避免击穿底层控件;拖拽中每个 move 上报
    `on_resize(新宽度)`。
- `Sidebar`:
  - `.on_resize(impl Fn(f32) -> Message + 'a)` 启用(存在即启用),
    `.min_width(f32)` 设拖拽下限,默认 180。
  - 拖拽内建于 `SidebarShell`(它持有 spring):按下记录 `press_x/press_width`,
    move 时算新宽度 → `SpringState::set()` 直接到位(精确跟手,无弹簧
    滞后)并上报消息;调用方把宽度存回 `.width(...)`。折叠/展开动画不受
    影响(非拖拽路径仍走 spring)。
  - 折叠态不启用:icon 折叠(目标是 48 的轨道)与 offcanvas 收起(目标是 0)
    时手柄关闭。
  - 拖拽上限:`可用行宽 - MIN_CONTENT_AREA`(下限仍保底 `min_width`)。
  - `collapsible(None)`(无动画包装)暂不支持拖拽,文档注明。
- `Settings`:
  - `.on_sidebar_resize(impl Fn(f32) -> Message + 'a)` 启用;侧栏列外包
    `ResizeEdge`,下限沿用 `sidebar_width` 的 120,上限
    `可用宽 - MIN_CONTENT_AREA`;`sidebar_width` 文档中"fixed rather than
    user-draggable"一句删除。
- 无状态约定不变:宽度始终由调用方持有,组件只在拖拽时上报新值。

## 测试

- `dragged_width` 数学:两侧锚定方向、上下限夹取、min>max 时的保底。
- Sidebar/Settings 各自渲染用例(带与不带 on_resize)。
- gallery:Sidebar 演示区与 Settings 演示区接入拖拽宽度(调用方状态持有)。
- 快照:空闲手柄不可见,现有三张 sidebar 快照不变。

## 验收

同仓库惯例:fmt(本改动文件)/ `clippy --all-targets -D warnings`(含
dock feature)/ `cargo test`、`cargo test --features dock` 全绿。
