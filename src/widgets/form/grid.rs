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

#![allow(dead_code)] // 移除于 Task 2（place_cells 被 Grid 消费后）

use std::collections::BTreeSet;

use crate::theme::Theme;
use iced::Element;

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
        let placement = match spec.start {
            Some(start) => {
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
            }
            None => {
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

#[cfg(test)]
mod tests {
    use super::{CellSpec, Placement, place_cells, resolve_span};

    fn spec(span: u16, start: Option<u16>, end: Option<u16>) -> CellSpec {
        CellSpec { span, start, end }
    }

    fn plain(count: u16) -> Vec<CellSpec> {
        (0..count).map(|_| spec(1, None, None)).collect()
    }

    fn placed(cells: &[Placement]) -> Vec<(usize, u16, u16)> {
        cells
            .iter()
            .map(|p| (p.row, p.column, p.span))
            .collect()
    }

    #[test]
    fn cells_flow_in_order_in_one_column() {
        let placed = placed(&place_cells(&plain(3), 1));
        assert_eq!(placed, vec![(0, 0, 1), (1, 0, 1), (2, 0, 1)]);
    }

    #[test]
    fn cells_pair_up_in_two_columns() {
        let placed = placed(&place_cells(&plain(4), 2));
        assert_eq!(
            placed,
            vec![(0, 0, 1), (0, 1, 1), (1, 0, 1), (1, 1, 1)]
        );
    }

    #[test]
    fn a_span_widens_a_cell_and_pushes_the_next_one_to_a_new_row() {
        let cells = vec![spec(2, None, None), spec(1, None, None), spec(1, None, None)];
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
}
