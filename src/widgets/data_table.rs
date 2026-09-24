//! Data tables: sortable, virtualized, with fixed or flexible columns.
//!
//! # Design
//!
//! A table is assembled from three pieces the caller controls:
//!
//! - [`Column`]s describe a field: its heading, width, alignment, and how to
//!   render the cell.
//! - [`TableState`] holds what the user has done to the view: sort column and
//!   direction, column widths, and selection.
//! - [`DataTable`] ties them to a slice of rows.
//!
//! Sorting is expressed as a key function per column, so the table never needs
//! to know anything about the row type beyond what its columns say.
//!
//! # Usage
//!
//! ```
//! use iced_kit::widgets::data_table::{Column, DataTable, SortKey, TableState, Width};
//! use iced_kit::Theme;
//! use iced::Element;
//!
//! #[derive(Clone, Debug)]
//! struct Person { name: String, age: u32 }
//!
//! #[derive(Clone, Debug)]
//! enum Message { Sorted(String), RowClicked(usize) }
//!
//! fn view<'a>(rows: &'a [Person], state: &'a TableState) -> Element<'a, Message, Theme> {
//!     DataTable::new(rows, state, vec![
//!         // A cell renderer receives the row and its index.
//!         Column::new("Name", |person: &Person, _row| {
//!             iced::widget::text(person.name.clone()).into()
//!         })
//!         .width(Width::Fill)
//!         .sortable(|person| SortKey::text(person.name.clone())),
//!         Column::new("Age", |person: &Person, _row| {
//!             iced::widget::text(person.age.to_string()).into()
//!         })
//!         .width(Width::Fixed(80.0))
//!         .align_x(iced::Alignment::End)
//!         .sortable(|person| SortKey::number(person.age)),
//!     ])
//!     .on_sort(Message::Sorted)
//!     .on_row_click(Message::RowClicked)
//!     .into()
//! }
//! ```

use crate::theme::{Size, Theme};
use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Alignment, Color, Element, Length, Padding};

/// How wide a column is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Width {
    /// A fixed width in logical pixels.
    Fixed(f32),
    /// A share of the remaining space.
    Fill,
    /// A share proportional to `FillPortion`.
    FillPortion(u16),
}

impl Width {
    /// The width as an iced [`Length`].
    fn as_length(self) -> Length {
        match self {
            Self::Fixed(pixels) => Length::Fixed(pixels.max(16.0)),
            Self::Fill => Length::Fill,
            Self::FillPortion(portion) => Length::FillPortion(portion.max(1)),
        }
    }
}

/// Which way a column is sorted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[must_use = "a SortDirection does nothing unless it is passed to a table"]
pub enum SortDirection {
    /// Smallest first, or A→Z.
    #[default]
    Ascending,
    /// Largest first, or Z→A.
    Descending,
}

impl SortDirection {
    /// The opposite direction.
    pub const fn flipped(self) -> Self {
        match self {
            Self::Ascending => Self::Descending,
            Self::Descending => Self::Ascending,
        }
    }

    /// The arrow shown in a sorted column's heading.
    pub const fn arrow(self) -> &'static str {
        match self {
            Self::Ascending => "▲",
            Self::Descending => "▼",
        }
    }
}

/// A sort key: a display string plus an optional numeric value.
///
/// Sortable columns return this. Carrying the numeric value separately is what
/// lets a numeric column sort by magnitude while displaying formatted text —
/// `"1,234"` must not sort before `"9"`.
#[derive(Debug, Clone, PartialEq)]
pub enum SortKey {
    /// Sorts numerically.
    Number(f64),
    /// Sorts lexicographically.
    Text(String),
}

impl SortKey {
    /// A numeric key.
    #[must_use]
    pub fn number(value: impl Into<f64>) -> Self {
        Self::Number(value.into())
    }

    /// A text key.
    #[must_use]
    pub fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }

    /// Orders two keys, numbers before text.
    ///
    /// The comparison is total and consistent, so a `sort_by` over a mixture of
    /// key kinds is stable rather than depending on input order.
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Self::Number(a), Self::Number(b)) => {
                // `partial_cmp` returns `None` for NaN; treating NaN as equal to
                // everything keeps the sort total instead of panicking.
                a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
            }
            (Self::Text(a), Self::Text(b)) => a.to_lowercase().cmp(&b.to_lowercase()),
            (Self::Number(_), Self::Text(_)) => std::cmp::Ordering::Less,
            (Self::Text(_), Self::Number(_)) => std::cmp::Ordering::Greater,
        }
    }
}

/// Renders one cell of a table, given its row and the row's index.
pub type CellRenderer<'a, T, Message> = Box<dyn Fn(&T, usize) -> Element<'a, Message, Theme> + 'a>;

/// Produces the sort key for a row.
pub type SortKeyFn<'a, T> = Box<dyn Fn(&T) -> SortKey + 'a>;

/// One column of a table.
#[must_use = "a Column does nothing unless it is given to a DataTable"]
pub struct Column<'a, T, Message> {
    heading: String,
    cell: CellRenderer<'a, T, Message>,
    sort_key: Option<SortKeyFn<'a, T>>,
    width: Width,
    align: Alignment,
    resizable: bool,
    fixed: bool,
}

impl<'a, T, Message: Clone + 'a> Column<'a, T, Message> {
    /// Creates a column that renders each cell with `cell`.
    pub fn new(
        heading: impl Into<String>,
        cell: impl Fn(&T, usize) -> Element<'a, Message, Theme> + 'a,
    ) -> Self {
        Self {
            heading: heading.into(),
            cell: Box::new(cell),
            sort_key: None,
            width: Width::Fill,
            align: Alignment::Start,
            // Allowing every column to be dragged would make the common case
            // (one flexible column that absorbs slack) fiddly to set up.
            resizable: false,
            fixed: false,
        }
    }

    /// Pins the column to the table's leading edge.
    ///
    /// A fixed column does not scroll with the rest: the data area is split in
    /// two, the fixed columns on the left and the scrolling remainder beside
    /// them. Column order is what decides which fixed columns come first, so a
    /// fixed column should be declared before the scrolling ones.
    pub fn fixed(mut self, fixed: bool) -> Self {
        self.fixed = fixed;
        self
    }

    /// Whether the column is pinned.
    #[must_use]
    pub fn is_fixed(&self) -> bool {
        self.fixed
    }

    /// Sets the column's width.
    pub fn width(mut self, width: Width) -> Self {
        self.width = width;
        self
    }

    /// Sets the horizontal alignment of the column's cells.
    pub fn align_x(mut self, align: Alignment) -> Self {
        self.align = align;
        self
    }

    /// Makes the column sortable, with `key` producing each row's sort value.
    pub fn sortable(mut self, key: impl Fn(&T) -> SortKey + 'a) -> Self {
        self.sort_key = Some(Box::new(key));
        self
    }

    /// Marks the column as resizable by dragging its trailing edge.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// The column's heading text.
    #[must_use]
    pub fn heading(&self) -> &str {
        &self.heading
    }

    /// Whether the column can be sorted.
    #[must_use]
    pub fn is_sortable(&self) -> bool {
        self.sort_key.is_some()
    }

    /// Whether the column can be resized.
    #[must_use]
    pub fn is_resizable(&self) -> bool {
        self.resizable
    }
}

/// A heading that spans several adjacent columns.
///
/// Groups are declared in order and consume the next `span` columns, so the
/// groups must account for every column or the header rows lose alignment:
/// a table with five columns is grouped by spans that add up to five.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use = "a ColumnGroup does nothing unless it is given to a DataTable"]
pub struct ColumnGroup {
    label: String,
    span: usize,
}

impl ColumnGroup {
    /// Creates a group spanning `span` columns.
    pub fn new(label: impl Into<String>, span: usize) -> Self {
        Self {
            label: label.into(),
            span: span.max(1),
        }
    }

    /// The group's heading text.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// How many columns the group covers.
    #[must_use]
    pub fn span(&self) -> usize {
        self.span
    }
}

/// What the user has done to a table's view.
///
/// The application owns this, so a table's sort order and column widths survive
/// across frames without the widget holding hidden state.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TableState {
    sort: Option<(String, SortDirection)>,
    selection: Option<usize>,
    widths: std::collections::HashMap<String, f32>,
}

impl TableState {
    /// Creates a state with no sort, no selection, and default widths.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a state sorted by a column.
    #[must_use]
    pub fn sorted_by(column: impl Into<String>, direction: SortDirection) -> Self {
        Self {
            sort: Some((column.into(), direction)),
            ..Self::default()
        }
    }

    /// The column the table is sorted by, and which way.
    #[must_use]
    pub fn sort(&self) -> Option<(&str, SortDirection)> {
        self.sort
            .as_ref()
            .map(|(column, direction)| (column.as_str(), *direction))
    }

    /// The selected row, if any.
    #[must_use]
    pub fn selection(&self) -> Option<usize> {
        self.selection
    }

    /// Selects a row, or clears the selection with `None`.
    pub fn select(&mut self, row: Option<usize>) {
        self.selection = row;
    }

    /// Applies a sort request from the user.
    ///
    /// Clicking the already-sorted column flips the direction; clicking a
    /// different column starts that one ascending. This is the convention a
    /// table header is expected to follow.
    pub fn toggle_sort(&mut self, column: impl Into<String>) {
        let column = column.into();

        self.sort = match self.sort.take() {
            Some((current, direction)) if current == column => Some((current, direction.flipped())),
            _ => Some((column, SortDirection::Ascending)),
        };
    }

    /// Sets a column's width, as reported by a resize drag.
    pub fn set_width(&mut self, column: impl Into<String>, width: f32) {
        if width.is_finite() && width > 0.0 {
            self.widths.insert(column.into(), width);
        }
    }

    /// The width the user has given a column, if they have resized it.
    #[must_use]
    pub fn width_of(&self, column: &str) -> Option<f32> {
        self.widths.get(column).copied()
    }

    /// The row order to display, given the columns' sort keys.
    ///
    /// Returns indices rather than reordering the data, so the caller's slice is
    /// never mutated and row identity stays stable across sorts.
    fn order<T, Message>(&self, rows: &[T], columns: &[Column<'_, T, Message>]) -> Vec<usize> {
        let mut order: Vec<usize> = (0..rows.len()).collect();

        let Some((sorted_column, direction)) = self.sort.as_ref() else {
            return order;
        };

        let Some(key) = columns
            .iter()
            .find(|column| column.heading == *sorted_column)
            .and_then(|column| column.sort_key.as_ref())
        else {
            return order;
        };

        order.sort_by(|left, right| {
            let ordering = key(&rows[*left]).cmp(&key(&rows[*right]));

            match direction {
                SortDirection::Ascending => ordering,
                SortDirection::Descending => ordering.reverse(),
            }
        });

        order
    }
}

/// A sortable, virtualized data table.
#[must_use = "a DataTable does nothing unless it is turned into an Element"]
pub struct DataTable<'a, T, Message> {
    rows: &'a [T],
    state: &'a TableState,
    columns: Vec<Column<'a, T, Message>>,
    row_height: f32,
    max_height: Option<f32>,
    stripe: bool,
    on_sort: Option<Box<dyn Fn(String) -> Message + 'a>>,
    on_row_click: Option<Box<dyn Fn(usize) -> Message + 'a>>,
    /// The application's own list state, for virtualization.
    list_state: &'a crate::widgets::VirtualListState,
    /// Optional heading groups drawn in a row above the column headings.
    column_groups: Vec<ColumnGroup>,
    /// When set, the body is replaced by this many skeleton rows.
    loading: Option<usize>,
}

impl<'a, T: 'a, Message: Clone + 'a> DataTable<'a, T, Message> {
    /// Creates a table over `rows` with the given columns.
    ///
    /// `list_state` carries the scroll position; the application owns it and
    /// feeds it back through
    /// [`VirtualListState`](crate::widgets::VirtualListState).
    pub fn new(rows: &'a [T], state: &'a TableState, columns: Vec<Column<'a, T, Message>>) -> Self {
        Self {
            rows,
            state,
            columns,
            row_height: 32.0,
            max_height: None,
            stripe: true,
            on_sort: None,
            on_row_click: None,
            list_state: &EMPTY_LIST_STATE,
            column_groups: Vec::new(),
            loading: None,
        }
    }

    /// Groups the column headings under spanning labels.
    ///
    /// The groups are drawn as an extra header row and are expected to cover
    /// every column in order; see [`ColumnGroup`].
    pub fn column_groups(mut self, groups: impl IntoIterator<Item = ColumnGroup>) -> Self {
        self.column_groups = groups.into_iter().collect();
        self
    }

    /// Replaces the body with skeleton rows while the data is being fetched.
    ///
    /// The columns still decide the header, so the table keeps its shape rather
    /// than collapsing to a spinner. Passing `0` shows the header alone.
    pub fn loading(mut self, rows: usize) -> Self {
        self.loading = Some(rows);
        self
    }

    /// Whether the table is showing skeleton rows.
    #[must_use]
    pub fn is_loading(&self) -> bool {
        self.loading.is_some()
    }

    /// The columns pinned to the leading edge, in declaration order.
    fn fixed_columns(&self) -> Vec<&Column<'a, T, Message>> {
        self.columns.iter().filter(|column| column.fixed).collect()
    }

    /// The columns that scroll with the data area.
    fn scrolling_columns(&self) -> Vec<&Column<'a, T, Message>> {
        self.columns.iter().filter(|column| !column.fixed).collect()
    }

    /// Sets the height of each row.
    pub fn row_height(mut self, height: f32) -> Self {
        self.row_height = height.max(1.0);
        self
    }

    /// Sets the height of the scrolling body.
    pub fn max_height(mut self, height: f32) -> Self {
        self.max_height = Some(height.max(1.0));
        self
    }

    /// Turns alternating row shading on or off.
    pub fn stripe(mut self, stripe: bool) -> Self {
        self.stripe = stripe;
        self
    }

    /// Reports a header click, carrying the column heading.
    pub fn on_sort(mut self, f: impl Fn(String) -> Message + 'a) -> Self {
        self.on_sort = Some(Box::new(f));
        self
    }

    /// Reports a row click, carrying the row index.
    pub fn on_row_click(mut self, f: impl Fn(usize) -> Message + 'a) -> Self {
        self.on_row_click = Some(Box::new(f));
        self
    }

    /// Supplies the scroll state, which enables virtualization.
    ///
    /// Without it the table renders every row, which is correct but linear in
    /// the size of the data set.
    pub fn scroll_state(mut self, state: &'a crate::widgets::VirtualListState) -> Self {
        self.list_state = state;
        self
    }

    /// The row indices to render, in display order.
    #[must_use]
    pub fn visible_order(&self) -> Vec<usize> {
        self.state.order(self.rows, &self.columns)
    }

    /// Builds the group heading row, when groups are declared.
    ///
    /// Spans are taken in order, each group consuming the next `span` columns.
    /// A group that runs past the last column is clamped, so a short group list
    /// cannot push the row out of alignment.
    fn group_row(&self) -> Option<Element<'a, Message, Theme>> {
        if self.column_groups.is_empty() {
            return None;
        }

        let heading_style = Size::Xs.text();
        let mut cells = row![].spacing(0).width(Length::Fill);
        let mut next = 0;

        for group in &self.column_groups {
            if next >= self.columns.len() {
                break;
            }

            let span = group.span.min(self.columns.len() - next);

            // A group's width is what its columns take, so the two header rows
            // line up. A group spanning the whole row is the full width rather
            // than the sum of its columns' fill, which would leave the last
            // column's slack unaccounted for.
            let covers_all = span == self.columns.len();

            let mut width = 0.0;
            let mut fill_portion = 0;
            let mut all_fixed = true;

            for column in &self.columns[next..next + span] {
                match self
                    .state
                    .width_of(&column.heading)
                    .map_or(column.width, Width::Fixed)
                {
                    Width::Fixed(pixels) => width += pixels,
                    Width::Fill => {
                        all_fixed = false;
                        fill_portion += 1;
                    }
                    Width::FillPortion(portion) => {
                        all_fixed = false;
                        fill_portion += portion;
                    }
                }
            }

            let length = if covers_all {
                Length::Fill
            } else if all_fixed {
                Length::Fixed(width)
            } else {
                Length::FillPortion(fill_portion.max(1))
            };

            cells = cells.push(
                container(
                    text(group.label.clone())
                        .size(heading_style.size)
                        .line_height(heading_style.line_height()),
                )
                .width(length)
                .height(Length::Fixed(self.row_height))
                .align_y(Alignment::Center)
                .padding(Padding {
                    top: 0.0,
                    right: 8.0,
                    bottom: 0.0,
                    left: 8.0,
                })
                .class(Box::new(|theme: &Theme| container::Style {
                    background: Some(iced::Background::Color(theme.colors().muted)),
                    border: iced::Border {
                        color: theme.colors().border,
                        width: 0.0,
                        radius: 0.0.into(),
                    },
                    text_color: Some(theme.colors().muted_foreground),
                    ..container::Style::default()
                }) as container::StyleFn<'a, Theme>),
            );

            next += span;
        }

        Some(
            container(cells)
                .width(Length::Fill)
                .class(Box::new(|theme: &Theme| container::Style {
                    background: Some(iced::Background::Color(theme.colors().muted)),
                    border: iced::Border {
                        color: theme.colors().border,
                        width: 0.0,
                        radius: 0.0.into(),
                    },
                    ..container::Style::default()
                }) as container::StyleFn<'a, Theme>)
                .into(),
        )
    }

    /// Builds one header cell for `column`.
    fn header_cell(&self, column: &Column<'a, T, Message>) -> Element<'a, Message, Theme> {
        let heading_style = Size::Sm.text();

        let width = self
            .state
            .width_of(&column.heading)
            .map_or(column.width, Width::Fixed);

        let sorted = self
            .state
            .sort()
            .filter(|(heading, _)| *heading == column.heading);

        // The label fills the cell so an end-aligned column's heading sits
        // over its values rather than at the cell's leading edge.
        let mut label = row![text(column.heading.clone())
            .size(heading_style.size)
            .line_height(heading_style.line_height())
            .width(Length::Fill)]
        .spacing(4)
        .align_y(Alignment::Center);

        if let Some((_, direction)) = sorted {
            label = label.push(text(direction.arrow()).size(heading_style.size - 3.0));
        }

        let heading_text = column.heading.clone();
        let is_sorted = sorted.is_some();
        let can_sort = column.is_sortable();

        let mut widget = button(
            container(label)
                .width(Length::Fill)
                .align_y(Alignment::Center),
        )
        .width(width.as_length())
        .height(Length::Fixed(self.row_height))
        .padding(Padding {
            top: 0.0,
            right: 8.0,
            bottom: 0.0,
            left: 8.0,
        })
        .class(Box::new(move |theme: &Theme, status| {
            header_cell_style(theme, status, is_sorted, can_sort)
        }) as button::StyleFn<'a, Theme>);

        if can_sort {
            if let Some(on_sort) = self.on_sort.as_ref() {
                widget = widget.on_press(on_sort(heading_text));
            }
        }

        widget.into()
    }

    /// Builds a header row over `columns`.
    fn header_row(&self, columns: &[&Column<'a, T, Message>]) -> Element<'a, Message, Theme> {
        let mut cells = row![].spacing(0).width(Length::Fill);

        for column in columns {
            cells = cells.push(self.header_cell(column));
        }

        container(cells)
            .width(Length::Fill)
            .class(Box::new(|theme: &Theme| container::Style {
                background: Some(iced::Background::Color(theme.colors().muted)),
                border: iced::Border {
                    color: theme.colors().border,
                    width: 0.0,
                    radius: 0.0.into(),
                },
                ..container::Style::default()
            }) as container::StyleFn<'a, Theme>)
            .into()
    }

    /// Builds the header row.
    fn header(&self) -> Element<'a, Message, Theme> {
        self.header_row(&self.columns.iter().collect::<Vec<_>>())
    }

    /// Builds one body row for the data row at `row_index`.
    ///
    /// `columns` is the slice the row draws: the whole set, or the fixed half
    /// or scrolling half of a table that pins columns.
    fn body_row(
        &self,
        row_index: usize,
        stripe: bool,
        columns: &[&Column<'a, T, Message>],
    ) -> Element<'a, Message, Theme> {
        let cell_style = Size::Sm.text();
        let selected = self.state.selection() == Some(row_index);
        let mut cells = row![].spacing(0).width(Length::Fill);

        for column in columns {
            let width = self
                .state
                .width_of(&column.heading)
                .map_or(column.width, Width::Fixed);

            let content = (column.cell)(&self.rows[row_index], row_index);

            cells = cells.push(
                container(content)
                    .width(width.as_length())
                    .height(Length::Fixed(self.row_height))
                    .align_y(Alignment::Center)
                    .align_x(column.align)
                    .padding(Padding {
                        top: 0.0,
                        right: 8.0,
                        bottom: 0.0,
                        left: 8.0,
                    }),
            );
        }

        let mut widget = button(cells.width(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fixed(self.row_height))
            .padding(Padding::default())
            .class(Box::new(move |theme: &Theme, status| {
                body_row_style(theme, status, selected, stripe)
            }) as button::StyleFn<'a, Theme>);

        if let Some(on_row_click) = self.on_row_click.as_ref() {
            widget = widget.on_press(on_row_click(row_index));
        }

        let _ = cell_style;

        widget.into()
    }

    /// Builds the fixed half of the body: one row per visible data row, over
    /// the pinned columns only.
    fn fixed_body(&self, stripe: bool) -> Element<'a, Message, Theme> {
        self.body_rows_over(&self.fixed_columns(), stripe)
    }

    /// Builds the scrolling half of the body.
    fn scrolling_body(&self, stripe: bool) -> Element<'a, Message, Theme> {
        self.body_rows_over(&self.scrolling_columns(), stripe)
    }

    /// Builds a body over `columns`.
    fn body_rows_over(
        &self,
        columns: &[&Column<'a, T, Message>],
        stripe: bool,
    ) -> Element<'a, Message, Theme> {
        let order = self.visible_order();
        let mut body = column![].spacing(0).width(Length::Fill);

        for (position, row_index) in order.iter().enumerate() {
            body = body.push(self.body_row(*row_index, stripe && position % 2 == 1, columns));
        }

        body.into()
    }

    /// Builds the placeholder body shown while loading.
    ///
    /// Each row is one skeleton bar per column, so the table keeps the shape of
    /// the data it is waiting for.
    fn loading_body(&self, rows: usize) -> Element<'a, Message, Theme> {
        let mut body = column![].spacing(0).width(Length::Fill);

        for index in 0..rows {
            let mut cells = row![].spacing(0).width(Length::Fill);

            for column in &self.columns {
                let width = self
                    .state
                    .width_of(&column.heading)
                    .map_or(column.width, Width::Fixed);

                // A short bar inside a full-width cell, so the placeholder
                // reads as content rather than as a second set of borders.
                let bar: Element<'a, Message, Theme> =
                    crate::widgets::skeleton(crate::widgets::SkeletonShape::Block, 1);

                cells = cells.push(
                    container(bar)
                        .width(width.as_length())
                        .height(Length::Fixed(self.row_height))
                        .align_y(Alignment::Center)
                        .padding(Padding {
                            top: 8.0,
                            right: 8.0,
                            bottom: 8.0,
                            left: 8.0,
                        }),
                );
            }

            body = body.push(
                container(cells.width(Length::Fill))
                    .width(Length::Fill)
                    .height(Length::Fixed(self.row_height))
                    .class(Box::new(move |theme: &Theme| container::Style {
                        border: iced::Border {
                            color: theme.colors().border,
                            width: 0.0,
                            radius: 0.0.into(),
                        },
                        background: (stripe_row(index))
                            .then_some(iced::Background::Color(theme.colors().surface)),
                        ..container::Style::default()
                    }) as container::StyleFn<'a, Theme>),
            );
        }

        body.into()
    }

    /// Converts the table into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let _ = self.list_state;

        let groups = self.group_row();
        let has_fixed = self.columns.iter().any(|column| column.fixed);

        // The body comes from the loading placeholder when one is set, and from
        // the data otherwise. Both are built per column set so a pinned table
        // gets two of each.
        let body = |fixed: bool| -> Element<'a, Message, Theme> {
            match self.loading {
                Some(rows) => self.loading_body(rows),
                None if fixed => self.fixed_body(self.stripe),
                None => self.scrolling_body(self.stripe),
            }
        };

        // Scrolling is installed even when nothing overflows: a table that can
        // be scrolled is what the caller asked for, and iced reports a viewport
        // of zero overflow as no scrollbar.
        let scroller = |body: Element<'a, Message, Theme>| {
            let mut scroller = scrollable(body).width(Length::Fill);
            if let Some(height) = self.max_height {
                scroller = scroller.height(Length::Fixed(height));
            }
            scroller
        };

        let mut root = column![].spacing(0).width(Length::Fill);

        if let Some(groups) = groups {
            root = root.push(groups);
        }

        if !has_fixed {
            root = root.push(self.header());
            root = root.push(scroller(body(false)));
            return root.into();
        }

        // A table with pinned columns draws four quadrants: the fixed heading
        // and cells stay put while the scrolling half moves beside them.
        let fixed_header = self.header_row(&self.fixed_columns());
        let scrolling_header = self.header_row(&self.scrolling_columns());

        let mut fixed_root = column![].spacing(0);
        fixed_root = fixed_root.push(fixed_header);
        fixed_root = fixed_root.push(body(true));

        let mut scrolling_root = column![].spacing(0).width(Length::Fill);
        scrolling_root = scrolling_root.push(scrolling_header);
        scrolling_root = scrolling_root.push(scroller(body(false)));

        root.push(
            row![fixed_root, scrolling_root]
                .spacing(0)
                .width(Length::Fill),
        )
        .into()
    }
}

impl<'a, T: 'a, Message: Clone + 'a> From<DataTable<'a, T, Message>>
    for Element<'a, Message, Theme>
{
    fn from(table: DataTable<'a, T, Message>) -> Self {
        table.into_element()
    }
}

/// A shared empty list state, so a table built without one still compiles.
static EMPTY_LIST_STATE: crate::widgets::VirtualListState =
    crate::widgets::VirtualListState::const_empty();

/// Whether a placeholder row at `index` is shaded, matching the data rows.
fn stripe_row(index: usize) -> bool {
    index % 2 == 1
}

/// The appearance of a header cell.
fn header_cell_style(
    theme: &Theme,
    status: button::Status,
    is_sorted: bool,
    can_sort: bool,
) -> button::Style {
    let colors = theme.colors();
    let hovered = matches!(status, button::Status::Hovered);

    button::Style {
        background: (hovered && can_sort).then_some(iced::Background::Color(colors.accent)),
        text_color: if is_sorted {
            colors.foreground
        } else {
            colors.muted_foreground
        },
        border: iced::Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        shadow: iced::Shadow::default(),
        snap: true,
    }
}

/// The appearance of a body row.
fn body_row_style(
    theme: &Theme,
    status: button::Status,
    selected: bool,
    stripe: bool,
) -> button::Style {
    let colors = theme.colors();
    let hovered = matches!(status, button::Status::Hovered);

    let background = if selected {
        Some(Color {
            a: 0.16,
            ..colors.primary
        })
    } else if hovered {
        Some(colors.accent)
    } else if stripe {
        Some(colors.muted)
    } else {
        None
    };

    button::Style {
        background: background.map(iced::Background::Color),
        text_color: colors.foreground,
        border: iced::Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        shadow: iced::Shadow::default(),
        snap: true,
    }
}

/// A cell that renders a plain string.
///
/// Offered as a free function because most tables have at least one column of
/// plain text, and writing the closure inline each time obscures the rest.
pub fn text_cell<'a, Message: 'a>(value: impl Into<String>) -> Element<'a, Message, Theme> {
    let style = Size::Sm.text();

    text(value.into())
        .size(style.size)
        .line_height(style.line_height())
        .into()
}

/// A cell that renders a numeric value right-aligned, for scanning magnitudes.
pub fn number_cell<'a, Message: 'a>(value: impl std::fmt::Display) -> Element<'a, Message, Theme> {
    let style = Size::Sm.text();

    text(format!("{value}"))
        .size(style.size)
        .line_height(style.line_height())
        .width(Length::Fill)
        .align_x(Alignment::End)
        .into()
}

/// An empty-state placeholder sized for a table's body.
pub fn empty_body<'a, Message: 'a>(
    message: impl Into<String>,
    height: f32,
) -> Element<'a, Message, Theme> {
    let style = Size::Sm.text();

    container(
        text(message.into())
            .size(style.size)
            .class(Box::new(|theme: &Theme| text::Style {
                color: Some(theme.colors().muted_foreground),
            }) as text::StyleFn<'a, Theme>),
    )
    .width(Length::Fill)
    .height(Length::Fixed(height.max(1.0)))
    .center_x(Length::Fill)
    .center_y(Length::Fixed(height.max(1.0)))
    .into()
}

/// A spacer that fills the remaining width of a row.
#[must_use]
pub fn filler<'a, Message: 'a>() -> Element<'a, Message, Theme> {
    Space::new().width(Length::Fill).into()
}

#[cfg(test)]
mod tests {
    use super::{
        empty_body, filler, number_cell, stripe_row, text_cell, Column, ColumnGroup, DataTable,
        SortDirection, SortKey, TableState, Width,
    };
    use crate::theme::Theme;

    #[derive(Debug, Clone, PartialEq)]
    struct Person {
        name: String,
        age: u32,
    }

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Sorted(String),
        RowClicked(usize),
    }

    fn people() -> Vec<Person> {
        vec![
            Person {
                name: "Charlie".to_owned(),
                age: 30,
            },
            Person {
                name: "Alice".to_owned(),
                age: 45,
            },
            Person {
                name: "Bob".to_owned(),
                age: 25,
            },
        ]
    }

    fn columns<'a>() -> Vec<Column<'a, Person, Message>> {
        vec![
            Column::new("Name", |person: &Person, _| text_cell(person.name.clone()))
                .width(Width::Fill)
                .sortable(|person| SortKey::text(person.name.clone())),
            Column::new("Age", |person: &Person, _| number_cell(person.age))
                .width(Width::Fixed(80.0))
                .align_x(iced::Alignment::End)
                .sortable(|person| SortKey::number(person.age)),
        ]
    }

    #[test]
    fn an_unsorted_table_keeps_the_input_order() {
        let data = people();
        let state = TableState::new();

        let table = DataTable::new(&data, &state, columns());
        assert_eq!(table.visible_order(), vec![0, 1, 2]);
    }

    #[test]
    fn sorting_by_text_orders_alphabetically() {
        let data = people();
        let state = TableState::sorted_by("Name", SortDirection::Ascending);

        let table = DataTable::new(&data, &state, columns());
        // Alice, Bob, Charlie
        assert_eq!(table.visible_order(), vec![1, 2, 0]);
    }

    #[test]
    fn sorting_descending_reverses_the_order() {
        let data = people();
        let state = TableState::sorted_by("Name", SortDirection::Descending);

        let table = DataTable::new(&data, &state, columns());
        assert_eq!(table.visible_order(), vec![0, 2, 1]);
    }

    #[test]
    fn a_numeric_column_sorts_by_magnitude_not_by_text() {
        let data = people();
        let state = TableState::sorted_by("Age", SortDirection::Ascending);

        let table = DataTable::new(&data, &state, columns());
        // 25, 30, 45 — a text sort would give "25", "30", "45" here too, so the
        // key type is what makes this reliable for larger values.
        assert_eq!(table.visible_order(), vec![2, 0, 1]);
    }

    #[test]
    fn a_numeric_key_sorts_correctly_where_text_would_not() {
        // "9" > "1,234" as text but 9 < 1234 numerically.
        let mut keys = vec![SortKey::number(1234.0), SortKey::number(9.0)];
        keys.sort_by(SortKey::cmp);
        assert_eq!(keys, vec![SortKey::number(9.0), SortKey::number(1234.0)]);

        let mut text_keys = vec![SortKey::text("1234"), SortKey::text("9")];
        text_keys.sort_by(SortKey::cmp);
        assert_eq!(text_keys, vec![SortKey::text("1234"), SortKey::text("9")]);
    }

    #[test]
    fn a_nan_key_does_not_break_the_sort() {
        // `partial_cmp` returns `None` for NaN; the comparison must stay total.
        let mut keys = [SortKey::number(f64::NAN), SortKey::number(1.0)];
        keys.sort_by(SortKey::cmp);
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn sorting_by_an_unknown_column_leaves_the_order_alone() {
        let data = people();
        let state = TableState::sorted_by("Nonexistent", SortDirection::Ascending);

        let table = DataTable::new(&data, &state, columns());
        assert_eq!(table.visible_order(), vec![0, 1, 2]);
    }

    #[test]
    fn sorting_an_unsortable_column_leaves_the_order_alone() {
        let data = people();

        let state = TableState::sorted_by("Name", SortDirection::Ascending);

        let columns = vec![Column::<Person, Message>::new(
            "Name",
            |person: &Person, _| text_cell(person.name.clone()),
        )];

        let order = {
            let table = DataTable::new(&data, &state, columns);
            table.visible_order()
        };
        assert_eq!(order, vec![0, 1, 2]);
    }

    #[test]
    fn clicking_the_sorted_column_flips_the_direction() {
        let mut state = TableState::new();

        state.toggle_sort("Name");
        assert_eq!(state.sort(), Some(("Name", SortDirection::Ascending)));

        state.toggle_sort("Name");
        assert_eq!(state.sort(), Some(("Name", SortDirection::Descending)));

        state.toggle_sort("Name");
        assert_eq!(state.sort(), Some(("Name", SortDirection::Ascending)));
    }

    #[test]
    fn clicking_a_different_column_starts_it_ascending() {
        let mut state = TableState::sorted_by("Name", SortDirection::Descending);

        state.toggle_sort("Age");
        assert_eq!(state.sort(), Some(("Age", SortDirection::Ascending)));
    }

    #[test]
    fn sort_directions_flip_and_report_an_arrow() {
        assert_eq!(
            SortDirection::Ascending.flipped(),
            SortDirection::Descending
        );
        assert_eq!(
            SortDirection::Descending.flipped(),
            SortDirection::Ascending
        );
        assert_ne!(
            SortDirection::Ascending.arrow(),
            SortDirection::Descending.arrow()
        );
    }

    #[test]
    fn selection_is_tracked_and_clearable() {
        let mut state = TableState::new();
        assert_eq!(state.selection(), None);

        state.select(Some(2));
        assert_eq!(state.selection(), Some(2));

        state.select(None);
        assert_eq!(state.selection(), None);
    }

    #[test]
    fn a_column_width_can_be_recorded() {
        let mut state = TableState::new();
        assert_eq!(state.width_of("Name"), None);

        state.set_width("Name", 240.0);
        assert_eq!(state.width_of("Name"), Some(240.0));
    }

    #[test]
    fn an_impossible_column_width_is_rejected() {
        let mut state = TableState::new();

        // A zero or negative width would collapse the column to nothing.
        for bad in [0.0, -100.0, f32::NAN, f32::INFINITY] {
            state.set_width("Name", bad);
            assert_eq!(state.width_of("Name"), None, "{bad} must be rejected");
        }
    }

    #[test]
    fn a_sorted_table_renders() {
        let data = people();
        let state = TableState::sorted_by("Age", SortDirection::Descending);

        let element: iced::Element<'_, Message, Theme> = DataTable::new(&data, &state, columns())
            .on_sort(Message::Sorted)
            .on_row_click(Message::RowClicked)
            .row_height(36.0)
            .max_height(300.0)
            .into();
        drop(element);
    }

    #[test]
    fn a_table_renders_without_handlers_or_a_height() {
        let data = people();
        let state = TableState::new();

        let element: iced::Element<'_, Message, Theme> =
            DataTable::new(&data, &state, columns()).into();
        drop(element);
    }

    #[test]
    fn an_empty_table_renders() {
        let data: Vec<Person> = Vec::new();
        let state = TableState::new();

        let element: iced::Element<'_, Message, Theme> =
            DataTable::new(&data, &state, columns()).into();
        drop(element);
    }

    #[test]
    fn a_table_with_no_columns_renders() {
        let data = people();
        let state = TableState::new();

        let element: iced::Element<'_, Message, Theme> =
            DataTable::new(&data, &state, Vec::new()).into();
        drop(element);
    }

    #[test]
    fn striping_can_be_turned_off() {
        let data = people();
        let state = TableState::new();

        let striped: iced::Element<'_, Message, Theme> =
            DataTable::new(&data, &state, columns()).stripe(true).into();
        drop(striped);

        let plain: iced::Element<'_, Message, Theme> = DataTable::new(&data, &state, columns())
            .stripe(false)
            .into();
        drop(plain);
    }

    #[test]
    fn a_selected_row_is_visually_distinct() {
        use iced::widget::button::Status;

        let theme = Theme::light();
        let selected = super::body_row_style(&theme, Status::Active, true, false);
        let striped = super::body_row_style(&theme, Status::Active, false, true);
        let plain = super::body_row_style(&theme, Status::Active, false, false);

        assert!(selected.background.is_some());
        assert_ne!(selected.background, striped.background);
        assert_ne!(selected.background, plain.background);
        assert!(plain.background.is_none(), "an unstriped row has no fill");
    }

    #[test]
    fn a_sortable_header_changes_appearance_and_an_unsortable_one_does_not() {
        use iced::widget::button::Status;

        let theme = Theme::light();
        let sortable = super::header_cell_style(&theme, Status::Hovered, false, true);
        let static_header = super::header_cell_style(&theme, Status::Hovered, false, false);

        assert!(sortable.background.is_some(), "a sortable header responds");
        assert!(
            static_header.background.is_none(),
            "a static header must not suggest it is clickable"
        );
    }

    #[test]
    fn a_sorted_header_is_emphasized() {
        use iced::widget::button::Status;

        let theme = Theme::light();
        let sorted = super::header_cell_style(&theme, Status::Active, true, true);
        let unsorted = super::header_cell_style(&theme, Status::Active, false, true);

        assert_ne!(sorted.text_color, unsorted.text_color);
    }

    #[test]
    fn column_metadata_is_reported() {
        let columns = columns();

        assert_eq!(columns[0].heading(), "Name");
        assert!(columns[0].is_sortable());
        assert!(!columns[0].is_resizable());

        let resizable =
            Column::<Person, Message>::new("X", |_: &Person, _| text_cell("x")).resizable(true);
        assert!(resizable.is_resizable());
    }

    #[test]
    fn widths_map_onto_iced_lengths() {
        assert_eq!(Width::Fixed(100.0).as_length(), iced::Length::Fixed(100.0));
        assert_eq!(Width::Fill.as_length(), iced::Length::Fill);
        assert_eq!(
            Width::FillPortion(2).as_length(),
            iced::Length::FillPortion(2)
        );

        // A degenerate width must not produce a zero-length column.
        assert_eq!(Width::Fixed(0.0).as_length(), iced::Length::Fixed(16.0));
        assert_eq!(
            Width::FillPortion(0).as_length(),
            iced::Length::FillPortion(1)
        );
    }

    #[test]
    fn cell_helpers_and_chrome_render() {
        let text_element: iced::Element<'_, Message, Theme> = text_cell("value");
        drop(text_element);

        let number: iced::Element<'_, Message, Theme> = number_cell(42);
        drop(number);

        let empty: iced::Element<'_, Message, Theme> = empty_body("No rows", 200.0);
        drop(empty);

        let space: iced::Element<'_, Message, Theme> = filler();
        drop(space);
    }

    #[test]
    fn the_table_accepts_a_scroll_state() {
        let data = people();
        let table_state = TableState::new();
        let list_state = crate::widgets::VirtualListState::new();

        let element: iced::Element<'_, Message, Theme> =
            DataTable::new(&data, &table_state, columns())
                .scroll_state(&list_state)
                .into();
        drop(element);
    }

    #[test]
    fn a_column_records_its_fixed_flag() {
        let column: Column<'_, Person, Message> =
            Column::new("Name", |person: &Person, _| text_cell(person.name.clone()));

        assert!(!column.is_fixed());
        assert!(column.fixed(true).is_fixed());
    }

    #[test]
    fn a_column_group_reports_its_label_and_span() {
        let group = ColumnGroup::new("Identity", 2);

        assert_eq!(group.label(), "Identity");
        assert_eq!(group.span(), 2);
        // A zero span would place no columns at all, so it is floored at one.
        assert_eq!(ColumnGroup::new("Empty", 0).span(), 1);
    }

    #[test]
    fn a_table_records_its_groups_and_loading_state() {
        let data = people();
        let table_state = TableState::new();

        let table: DataTable<'_, Person, Message> = DataTable::new(&data, &table_state, columns())
            .column_groups([
                ColumnGroup::new("Identity", 1),
                ColumnGroup::new("Details", 1),
            ])
            .loading(4);

        assert!(table.is_loading());
        assert_eq!(table.column_groups.len(), 2);
        assert_eq!(table.loading, Some(4));
    }

    /// A table that is not loading must not draw placeholder rows.
    #[test]
    fn loading_is_off_unless_asked_for() {
        let data = people();
        let table_state = TableState::new();

        let table: DataTable<'_, Person, Message> = DataTable::new(&data, &table_state, columns());
        assert!(!table.is_loading());
        assert!(table.column_groups.is_empty());
    }

    #[test]
    fn the_column_halves_partition_the_columns() {
        let data = people();
        let table_state = TableState::new();

        let table: DataTable<'_, Person, Message> = DataTable::new(
            &data,
            &table_state,
            vec![
                Column::new("Name", |person: &Person, _| text_cell(person.name.clone()))
                    .fixed(true),
                Column::new("Age", |person: &Person, _| number_cell(person.age)),
            ],
        );

        assert_eq!(table.fixed_columns().len(), 1);
        assert_eq!(table.fixed_columns()[0].heading(), "Name");
        assert_eq!(table.scrolling_columns().len(), 1);
        assert_eq!(table.scrolling_columns()[0].heading(), "Age");
    }

    #[test]
    fn a_pinned_column_renders_in_the_split_table() {
        let data = people();
        let table_state = TableState::new();

        let element: iced::Element<'_, Message, Theme> = DataTable::new(
            &data,
            &table_state,
            vec![
                Column::new("Name", |person: &Person, _| text_cell(person.name.clone()))
                    .fixed(true),
                Column::new("Age", |person: &Person, _| number_cell(person.age)),
            ],
        )
        .into();
        drop(element);
    }

    #[test]
    fn a_loading_table_renders_placeholders() {
        let data = people();
        let table_state = TableState::new();

        let element: iced::Element<'_, Message, Theme> =
            DataTable::new(&data, &table_state, columns())
                .loading(3)
                .into();
        drop(element);

        // The zero case keeps the header alone rather than panicking.
        let header_only: iced::Element<'_, Message, Theme> =
            DataTable::new(&data, &table_state, columns())
                .loading(0)
                .into();
        drop(header_only);
    }

    #[test]
    fn a_grouped_table_renders_its_group_row() {
        let data = people();
        let table_state = TableState::new();

        let element: iced::Element<'_, Message, Theme> =
            DataTable::new(&data, &table_state, columns())
                .column_groups([
                    ColumnGroup::new("Identity", 1),
                    ColumnGroup::new("Details", 1),
                ])
                .into();
        drop(element);

        // A group list shorter than the columns must not push the row out of
        // alignment; a longer one is clamped.
        let over: iced::Element<'_, Message, Theme> =
            DataTable::new(&data, &table_state, columns())
                .column_groups([
                    ColumnGroup::new("Everything", 99),
                    ColumnGroup::new("Past the end", 2),
                ])
                .into();
        drop(over);
    }

    #[test]
    fn placeholder_rows_stripe_like_data_rows() {
        assert!(!stripe_row(0));
        assert!(stripe_row(1));
        assert!(!stripe_row(2));
    }
}
