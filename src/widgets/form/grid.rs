//! A row-major auto-flow grid with equal columns.
//!
//! iced has no CSS grid, so the form brings its own: cells declare where they
//! want to sit (a column span, an optional start column, an optional end
//! line), the grid resolves those into positions, and lays the cells out at
//! the width their span covers.
//!
//! Placement is a pure function so the resolution rules — explicit placements
//! first, the rest flowing in order and never going back to fill a hole — can
//! be tested without rendering.

use std::collections::BTreeSet;

use crate::theme::Theme;
use iced::advanced::layout::{self, Limits, Node};
use iced::advanced::widget::{tree, Operation};
use iced::advanced::{mouse, Clipboard, Shell, Widget};
use iced::{Element, Length, Point, Rectangle, Size, Vector};

/// One slot in the grid: an element plus where it wants to sit.
pub(crate) struct GridCell<'a, Message> {
    /// How many columns the cell covers. Clamped to the column count.
    pub(crate) span: u16,
    /// The 1-based column the cell starts in, as CSS `grid-column-start`.
    pub(crate) start: Option<u16>,
    /// The 1-based line the cell must not reach, as CSS `grid-column-end`:
    /// a cell ending at line 3 occupies 0-based columns below 2.
    pub(crate) end: Option<u16>,
    pub(crate) content: Element<'a, Message, Theme>,
}

/// What the placement algorithm sees of one cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CellSpec {
    span: u16,
    start: Option<u16>,
    end: Option<u16>,
}

/// Where the grid decided one cell sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Placement {
    /// 0-based row index.
    row: usize,
    /// 0-based column index.
    column: u16,
    span: u16,
}

/// Resolves where every cell sits.
///
/// One pass in document order, so the reading order never doubles back: a
/// cell locked to a column behind the cursor drops to a new row, and the
/// auto-flow cursor only ever advances — a hole stays a hole (CSS sparse
/// auto-flow).
fn place_cells(cells: &[CellSpec], columns: u16) -> Vec<Placement> {
    let columns = columns.max(1);
    let mut occupied: BTreeSet<(usize, u16)> = BTreeSet::new();
    let mut cursor = (0usize, 0u16);
    let mut placements = Vec::with_capacity(cells.len());

    for spec in cells {
        let span = resolve_span(*spec, columns);
        let placement = if let Some(start) = spec.start {
            // The cell is locked to this column (1-based), clamped into
            // the grid. A locked column behind the cursor starts a new
            // row rather than doubling back over placed cells.
            let column = start.min(columns.saturating_sub(span) + 1) - 1;
            if column < cursor.1 {
                cursor = (cursor.0 + 1, column);
            } else {
                cursor.1 = column;
            }
            while !is_free(&occupied, cursor.0, cursor.1, span) {
                cursor = (cursor.0 + 1, column);
            }
            Placement {
                row: cursor.0,
                column: cursor.1,
                span,
            }
        } else {
            // A cell ending at line `end` occupies 0-based columns below
            // `end - span - 1`, so that is the furthest its column may go.
            let last = spec
                .end
                .map_or(columns - span, |end| end.saturating_sub(span + 1))
                .min(columns - span);
            loop {
                if cursor.1 + span > columns {
                    cursor = (cursor.0 + 1, 0);
                    continue;
                }
                // The cell may start earlier in this row to honour its
                // end line, but never in a row already passed.
                if cursor.1 > last {
                    cursor.1 = last;
                }
                if is_free(&occupied, cursor.0, cursor.1, span) {
                    break Placement {
                        row: cursor.0,
                        column: cursor.1,
                        span,
                    };
                }
                if cursor.1 >= last {
                    cursor = (cursor.0 + 1, 0);
                } else {
                    cursor.1 += 1;
                }
            }
        };
        mark(&mut occupied, &placement);
        cursor = (placement.row, placement.column + placement.span);
        placements.push(placement);
    }

    placements
}

fn is_free(occupied: &BTreeSet<(usize, u16)>, row: usize, column: u16, span: u16) -> bool {
    (0..span).all(|offset| !occupied.contains(&(row, column + offset)))
}

fn mark(occupied: &mut BTreeSet<(usize, u16)>, placement: &Placement) {
    for offset in 0..placement.span {
        occupied.insert((placement.row, placement.column + offset));
    }
}

/// A cell's width in columns: an explicit start *and* end line fixes it
/// (line numbers, so lines 1..3 are two columns); otherwise the span stands,
/// clamped so the cell always fits the grid.
fn resolve_span(spec: CellSpec, columns: u16) -> u16 {
    match (spec.start, spec.end) {
        (Some(start), Some(end)) => end.saturating_sub(start).clamp(1, columns),
        (None, Some(end)) => spec.span.min(end.saturating_sub(1)).clamp(1, columns),
        _ => spec.span.clamp(1, columns),
    }
}

/// The form's grid: equal columns, rows sized by their tallest cell.
pub(crate) struct Grid<'a, Message> {
    columns: usize,
    row_spacing: f32,
    column_spacing: f32,
    pub(super) cells: Vec<GridCell<'a, Message>>,
}

impl<'a, Message> Grid<'a, Message> {
    pub(crate) fn new(columns: usize, row_spacing: f32, column_spacing: f32) -> Self {
        Self {
            columns: columns.max(1),
            row_spacing,
            column_spacing,
            cells: Vec::new(),
        }
    }

    pub(crate) fn push(mut self, cell: GridCell<'a, Message>) -> Self {
        self.cells.push(cell);
        self
    }
}

impl<'a, Message: 'a> From<Grid<'a, Message>> for Element<'a, Message, Theme> {
    fn from(grid: Grid<'a, Message>) -> Self {
        Element::new(grid)
    }
}

impl<Message> Widget<Message, Theme, iced::Renderer> for Grid<'_, Message> {
    fn size(&self) -> Size<Length> {
        // A form fills the width it is given, like the reference's `w_full`.
        Size::new(Length::Fill, Length::Shrink)
    }

    fn children(&self) -> Vec<tree::Tree> {
        self.cells
            .iter()
            .map(|cell| tree::Tree::new(&cell.content))
            .collect()
    }

    fn diff(&self, tree: &mut tree::Tree) {
        let contents: Vec<&Element<'_, Message, Theme>> =
            self.cells.iter().map(|cell| &cell.content).collect();
        tree.diff_children(&contents);
    }

    fn layout(
        &mut self,
        tree: &mut tree::Tree,
        renderer: &iced::Renderer,
        limits: &Limits,
    ) -> Node {
        let width = limits.max().width;
        let columns = self.columns as u16;

        let specs: Vec<CellSpec> = self
            .cells
            .iter()
            .map(|cell| CellSpec {
                span: cell.span,
                start: cell.start,
                end: cell.end,
            })
            .collect();
        let placements = place_cells(&specs, columns);

        let usable = (width - self.column_spacing * f32::from(columns - 1)).max(0.0);
        let column_width = usable / f32::from(columns);
        // A spanned cell covers its columns *and* the gaps between them.
        let spanned_width =
            |span: u16| column_width * f32::from(span) + self.column_spacing * f32::from(span - 1);

        // Lay every cell out at its spanned width, recording row heights.
        let mut heights: Vec<f32> = Vec::new();
        let mut laid_out = Vec::with_capacity(self.cells.len());
        for (index, cell) in self.cells.iter_mut().enumerate() {
            let placement = placements[index];
            let cell_width = spanned_width(placement.span);
            let cell_limits = Limits::new(
                Size::new(cell_width, 0.0),
                Size::new(cell_width, f32::INFINITY),
            );
            let node = cell.content.as_widget_mut().layout(
                &mut tree.children[index],
                renderer,
                &cell_limits,
            );
            while heights.len() <= placement.row {
                heights.push(0.0);
            }
            heights[placement.row] = heights[placement.row].max(node.size().height);
            laid_out.push((placement, node));
        }

        // Rows top to bottom, cells left to right within their row.
        let mut row_tops = Vec::with_capacity(heights.len());
        let mut y = 0.0;
        for height in &heights {
            row_tops.push(y);
            y += height + self.row_spacing;
        }
        let total_height = (y - self.row_spacing).max(0.0);

        let children = laid_out
            .into_iter()
            .map(|(placement, node)| {
                let x = f32::from(placement.column) * (column_width + self.column_spacing);
                node.move_to(Point::new(x, row_tops[placement.row]))
            })
            .collect();

        Node::with_children(Size::new(width, total_height), children)
    }

    fn operate(
        &mut self,
        tree: &mut tree::Tree,
        layout: layout::Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        let children: Vec<layout::Layout<'_>> = layout.children().collect();
        for (index, cell) in self.cells.iter_mut().enumerate() {
            let Some(child_layout) = children.get(index).copied() else {
                break;
            };
            cell.content.as_widget_mut().operate(
                &mut tree.children[index],
                child_layout,
                renderer,
                operation,
            );
        }
    }

    fn update(
        &mut self,
        tree: &mut tree::Tree,
        event: &iced::Event,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let children: Vec<layout::Layout<'_>> = layout.children().collect();
        for (index, cell) in self.cells.iter_mut().enumerate() {
            let Some(child_layout) = children.get(index).copied() else {
                break;
            };
            cell.content.as_widget_mut().update(
                &mut tree.children[index],
                event,
                child_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }
    }

    fn draw(
        &self,
        tree: &tree::Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &iced::advanced::renderer::Style,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let children: Vec<layout::Layout<'_>> = layout.children().collect();
        for (index, cell) in self.cells.iter().enumerate() {
            let Some(child_layout) = children.get(index).copied() else {
                break;
            };
            cell.content.as_widget().draw(
                &tree.children[index],
                renderer,
                theme,
                style,
                child_layout,
                cursor,
                viewport,
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &tree::Tree,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let children: Vec<layout::Layout<'_>> = layout.children().collect();
        for (index, cell) in self.cells.iter().enumerate() {
            let Some(child_layout) = children.get(index).copied() else {
                break;
            };
            let interaction = cell.content.as_widget().mouse_interaction(
                &tree.children[index],
                child_layout,
                cursor,
                viewport,
                renderer,
            );
            if interaction != mouse::Interaction::None {
                return interaction;
            }
        }
        mouse::Interaction::None
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut tree::Tree,
        layout: layout::Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, iced::Renderer>> {
        // Mirrors `overlay::from_children`, which the cells cannot be handed
        // to directly: each cell wraps its element in a grid placement, and
        // the helper wants a bare `&mut [Element]`.
        let overlays = self
            .cells
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
            .filter_map(|((cell, tree), layout)| {
                cell.content
                    .as_widget_mut()
                    .overlay(tree, layout, renderer, viewport, translation)
            })
            .collect::<Vec<_>>();

        (!overlays.is_empty())
            .then(|| iced::advanced::overlay::Group::with_children(overlays).overlay())
    }
}

#[cfg(test)]
mod tests {
    use super::{place_cells, resolve_span, CellSpec, Placement};

    fn spec(span: u16, start: Option<u16>, end: Option<u16>) -> CellSpec {
        CellSpec { span, start, end }
    }

    fn plain(count: u16) -> Vec<CellSpec> {
        (0..count).map(|_| spec(1, None, None)).collect()
    }

    fn placed(cells: &[Placement]) -> Vec<(usize, u16, u16)> {
        cells.iter().map(|p| (p.row, p.column, p.span)).collect()
    }

    #[test]
    fn cells_flow_in_order_in_one_column() {
        let placed = placed(&place_cells(&plain(3), 1));
        assert_eq!(placed, vec![(0, 0, 1), (1, 0, 1), (2, 0, 1)]);
    }

    #[test]
    fn cells_pair_up_in_two_columns() {
        let placed = placed(&place_cells(&plain(4), 2));
        assert_eq!(placed, vec![(0, 0, 1), (0, 1, 1), (1, 0, 1), (1, 1, 1)]);
    }

    #[test]
    fn a_span_widens_a_cell_and_pushes_the_next_one_to_a_new_row() {
        let cells = vec![
            spec(2, None, None),
            spec(1, None, None),
            spec(1, None, None),
        ];
        let placed = placed(&place_cells(&cells, 2));
        // The wide cell takes the whole first row, so the next flows on.
        assert_eq!(placed, vec![(0, 0, 2), (1, 0, 1), (1, 1, 1)]);
    }

    #[test]
    fn an_explicit_start_sits_its_cell_and_keeps_the_flow_going() {
        let cells = vec![
            spec(1, None, None),
            spec(1, Some(1), None),
            spec(1, None, None),
        ];
        let placed = placed(&place_cells(&cells, 2));
        // The explicit cell drops to row 1: its column lies behind the
        // cursor, and doubling back would break the reading order. The
        // third cell follows it, leaving the hole beside the first —
        // sparse auto-flow never goes back.
        assert_eq!(placed, vec![(0, 0, 1), (1, 0, 1), (1, 1, 1)]);
    }

    #[test]
    fn an_end_line_caps_where_the_flow_may_place_a_cell() {
        let cells = vec![spec(1, None, None), spec(1, None, Some(2))];
        let placed = placed(&place_cells(&cells, 2));
        // Column 1 is free, but a cell ending at line 2 must sit in column 0;
        // that slot is taken, so it drops to the next row.
        assert_eq!(placed, vec![(0, 0, 1), (1, 0, 1)]);
    }

    #[test]
    fn a_span_clamps_to_the_column_count() {
        let placed = placed(&place_cells(&[spec(5, None, None)], 2));
        assert_eq!(placed, vec![(0, 0, 2)]);
    }

    #[test]
    fn an_explicit_start_clamps_into_the_grid() {
        let placed = placed(&place_cells(&[spec(1, Some(5), None)], 2));
        assert_eq!(placed, vec![(0, 1, 1)]);
    }

    #[test]
    fn an_explicit_start_and_end_line_fix_the_span() {
        assert_eq!(resolve_span(spec(1, Some(1), Some(3)), 4), 2);
        assert_eq!(resolve_span(spec(3, Some(2), Some(3)), 4), 1);
        // An end line alone caps the span: a cell ending at line 2 cannot be
        // wider than one column.
        assert_eq!(resolve_span(spec(3, None, Some(2)), 4), 1);
    }

    #[test]
    fn zero_columns_is_treated_as_one() {
        let placed = placed(&place_cells(&plain(2), 0));
        assert_eq!(placed, vec![(0, 0, 1), (1, 0, 1)]);
    }

    #[test]
    fn a_grid_constructs_into_an_element() {
        use super::{Grid, GridCell};
        use crate::theme::Theme;
        use iced::widget::text;

        let grid: Grid<'_, ()> = Grid::new(2, 8.0, 24.0).push(GridCell {
            span: 1,
            start: None,
            end: None,
            content: text("a").into(),
        });
        let element: iced::Element<'_, (), Theme, iced::Renderer> = grid.into();
        drop(element);
    }
}
