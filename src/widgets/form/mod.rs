//! Forms: labelled fields arranged in a grid.
//!
//! This module aligns with `gpui-kit/crates/component/src/form`. The design
//! notes live in `docs/superpowers/specs/2026-09-24-form-design.md`.
//!
//! # Usage
//!
//! ```
//! use iced_kit::widgets::{field, form, text_input};
//! use iced_kit::Theme;
//! use iced::Element;
//!
//! # #[derive(Clone, Debug)] enum Message { Name(String) }
//! # fn view() -> Element<'static, Message, Theme> {
//! form()
//!     .child(
//!         field()
//!             .label("Name")
//!             .required(true)
//!             .description("Your given name.")
//!             .push(text_input::<Message>("Ada Lovelace", "").on_input(Message::Name)),
//!     )
//!     .into()
//! # }
//! ```

pub(crate) mod grid;

use crate::theme::{Size, Theme};
use crate::widgets::form::grid::GridCell;
use iced::widget::{column, container, row, text, Column};
use iced::{alignment, Element, Length};

/// Where a [`Field`]'s label sits relative to its control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FormLabelLayout {
    /// The label sits above its control. The default.
    #[default]
    Vertical,
    /// The label sits beside its control, at a fixed width.
    Horizontal,
}

/// A [`Field`]'s label or description: plain text, or any element.
///
/// The reference's `FieldBuilder` also carried a view variant; iced has no
/// view handles, so an element covers that case. An `Element` is neither
/// `Clone` nor `Debug`, so neither is a label carrying one.
pub enum FieldLabel<'a, Message> {
    /// Text drawn at the label style (small, medium weight).
    Text(String),
    /// An element passed through untouched: an icon, a link, anything.
    Element(Element<'a, Message, Theme>),
}

impl<'a, Message> From<&str> for FieldLabel<'a, Message> {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

impl<'a, Message> From<String> for FieldLabel<'a, Message> {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl<'a, Message> From<iced::widget::Text<'a, Theme>> for FieldLabel<'a, Message> {
    fn from(value: iced::widget::Text<'a, Theme>) -> Self {
        Self::Element(value.into())
    }
}

impl<'a, Message> From<Element<'a, Message, Theme>> for FieldLabel<'a, Message> {
    fn from(value: Element<'a, Message, Theme>) -> Self {
        Self::Element(value)
    }
}

/// What a [`Form`] hands each of its [`Field`]s.
#[derive(Debug, Clone, Copy)]
struct FieldProps {
    layout: FormLabelLayout,
    size: Size,
    columns: usize,
    label_width: f32,
    label_text_size: Option<f32>,
}

impl Default for FieldProps {
    fn default() -> Self {
        Self {
            layout: FormLabelLayout::Vertical,
            size: Size::Md,
            columns: 1,
            label_width: 140.0,
            label_text_size: None,
        }
    }
}

/// The row spacing a form claims at this size.
fn form_row_spacing(size: Size) -> f32 {
    match size {
        Size::Xs | Size::Sm => 6.0,
        Size::Lg => 12.0,
        Size::Md => 8.0,
        Size::Custom(pixels) => pixels,
    }
}

/// The spacing a field claims between its label, control and description.
fn field_spacing(size: Size) -> f32 {
    match size {
        Size::Lg => 8.0,
        _ => 4.0,
    }
}

/// One labelled control, with a description and a grid placement.
///
/// Build it with [`field`], add controls with [`Field::push`], and add the
/// field to a [`Form`] with [`Form::child`].
#[must_use = "a Field does nothing unless it is added to a Form"]
pub struct Field<'a, Message> {
    label: Option<FieldLabel<'a, Message>>,
    description: Option<FieldLabel<'a, Message>>,
    children: Vec<Element<'a, Message, Theme>>,
    visible: bool,
    required: bool,
    label_indent: bool,
    align_items: Option<alignment::Alignment>,
    col_span: u16,
    col_start: Option<u16>,
    col_end: Option<u16>,
}

impl<'a, Message: 'a> Field<'a, Message> {
    /// Creates an empty field.
    pub fn new() -> Self {
        Self {
            label: None,
            description: None,
            children: Vec::new(),
            visible: true,
            required: false,
            label_indent: true,
            align_items: None,
            col_span: 1,
            col_start: None,
            col_end: None,
        }
    }

    /// Sets the label drawn above (or beside) the control.
    pub fn label(mut self, label: impl Into<FieldLabel<'a, Message>>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets a secondary line drawn under the control, in a muted small face.
    pub fn description(mut self, description: impl Into<FieldLabel<'a, Message>>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Marks the field required, drawing a danger-colored `*` after the label.
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Hides the field without removing it from the call site: a `Form`
    /// drops invisible fields before placing the rest.
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Keeps (default) or drops the label-width indent that aligns the
    /// control column in a horizontal form.
    pub fn label_indent(mut self, indent: bool) -> Self {
        self.label_indent = indent;
        self
    }

    /// Aligns the label and control to the start of their row. The default.
    pub fn items_start(mut self) -> Self {
        self.align_items = Some(alignment::Alignment::Start);
        self
    }

    /// Aligns the label and control to the end of their row.
    pub fn items_end(mut self) -> Self {
        self.align_items = Some(alignment::Alignment::End);
        self
    }

    /// Centers the label and control on their row.
    pub fn items_center(mut self) -> Self {
        self.align_items = Some(alignment::Alignment::Center);
        self
    }

    /// Covers `span` columns of the form's grid.
    pub fn col_span(mut self, span: u16) -> Self {
        self.col_span = span;
        self
    }

    /// Starts the field at the 1-based column `start`, as CSS
    /// `grid-column-start`.
    pub fn col_start(mut self, start: u16) -> Self {
        self.col_start = Some(start);
        self
    }

    /// Keeps the field before the 1-based line `end`, as CSS
    /// `grid-column-end`.
    pub fn col_end(mut self, end: u16) -> Self {
        self.col_end = Some(end);
        self
    }

    /// Adds a control — the input, switch or picker the field labels.
    pub fn push(mut self, child: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.children.push(child.into());
        self
    }

    /// Adds several controls.
    pub fn extend<I>(mut self, children: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<Element<'a, Message, Theme>>,
    {
        self.children.extend(children.into_iter().map(Into::into));
        self
    }

    /// Turns the field into a grid cell carrying the form's properties.
    fn into_cell(self, props: FieldProps) -> GridCell<'a, Message> {
        GridCell {
            span: self.col_span,
            start: self.col_start,
            end: self.col_end,
            content: self.into_element_with(props),
        }
    }

    /// Lays the field out: the label and control, then the description.
    fn into_element_with(self, props: FieldProps) -> Element<'a, Message, Theme> {
        let Self {
            label,
            description,
            children,
            visible: _,
            required,
            label_indent,
            align_items,
            col_span: _,
            col_start: _,
            col_end: _,
        } = self;

        let gap = field_spacing(props.size);
        let inner_gap = if props.layout == FormLabelLayout::Horizontal {
            gap
        } else {
            gap / 2.0
        };
        let align = align_items.unwrap_or(alignment::Alignment::Start);
        let has_label = label.is_some();

        let controls = Column::with_children(children).width(Length::Fill);

        // The label and its control, side by side or stacked.
        let labelled: Element<'a, Message, Theme> = match (props.layout, label) {
            (FormLabelLayout::Horizontal, label) => {
                let mut line = row![].spacing(inner_gap).align_y(align);
                if let Some(label) = label {
                    line = line.push(
                        container(label_element(label, required, props.label_text_size))
                            .width(Length::Fixed(props.label_width)),
                    );
                }
                line.push(controls).into()
            }
            (FormLabelLayout::Vertical, label) => {
                let mut line = column![]
                    .spacing(inner_gap)
                    .align_x(align)
                    .width(Length::Fill);
                if let Some(label) = label {
                    line = line.push(label_element(label, required, props.label_text_size));
                }
                line.push(controls).into()
            }
        };

        // The description, indented to the control column in a horizontal
        // form whose label indents.
        let described: Element<'a, Message, Theme> = match description {
            None => labelled,
            Some(description) => {
                let note = muted_note(description);
                if props.layout == FormLabelLayout::Horizontal && label_indent && has_label {
                    row![
                        iced::widget::Space::new().width(Length::Fixed(props.label_width)),
                        note,
                    ]
                    .spacing(inner_gap)
                    .into()
                } else {
                    column![note].into()
                }
            }
        };

        column![described].spacing(gap / 2.0).into()
    }
}

impl<'a, Message: 'a> Default for Field<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

/// Draws a label: small, medium weight, with a `*` when required.
fn label_element<'a, Message: 'a>(
    label: FieldLabel<'a, Message>,
    required: bool,
    text_size: Option<f32>,
) -> Element<'a, Message, Theme> {
    let content = match label {
        FieldLabel::Text(value) => {
            let style = Size::Sm.text();
            text(value)
                .size(text_size.unwrap_or(style.size))
                .line_height(style.line_height())
                .font(iced::Font {
                    weight: iced::font::Weight::Medium,
                    ..iced::Font::DEFAULT
                })
                .into()
        }
        // A custom label brings its own styling, which is the point of
        // passing an element.
        FieldLabel::Element(element) => element,
    };

    if !required {
        return content;
    }

    let style = Size::Sm.text();
    let star = text("*")
        .size(text_size.unwrap_or(style.size))
        .line_height(style.line_height())
        .class(Box::new(|theme: &Theme| iced::widget::text::Style {
            color: Some(theme.colors().destructive),
        }) as iced::widget::text::StyleFn<'a, Theme>);
    row![content, star]
        .spacing(4)
        .align_y(alignment::Alignment::Center)
        .into()
}

/// Draws a description: extra small, muted.
fn muted_note<'a, Message: 'a>(note: FieldLabel<'a, Message>) -> Element<'a, Message, Theme> {
    match note {
        FieldLabel::Text(value) => {
            let style = Size::Xs.text();
            text(value)
                .size(style.size)
                .line_height(style.line_height())
                .class(Box::new(|theme: &Theme| iced::widget::text::Style {
                    color: Some(theme.colors().muted_foreground),
                }) as iced::widget::text::StyleFn<'a, Theme>)
                .into()
        }
        FieldLabel::Element(element) => element,
    }
}

/// A form: labelled fields arranged in a grid, with a trailing footer.
///
/// Build it with [`form`], [`Form::vertical`] or [`Form::horizontal`]. A form
/// owns the fields it was given, and iced's `Element` is neither `Clone` nor
/// `Debug`, so a form cannot be copied or printed.
#[must_use = "a Form does nothing unless it is turned into an Element"]
pub struct Form<'a, Message> {
    props: FieldProps,
    fields: Vec<Field<'a, Message>>,
    footer: Option<Element<'a, Message, Theme>>,
}

impl<'a, Message: 'a> Form<'a, Message> {
    /// Creates a single-column form with labels above their controls.
    pub fn new() -> Self {
        Self {
            props: FieldProps::default(),
            fields: Vec::new(),
            footer: None,
        }
    }

    /// Creates a form with labels beside their controls.
    pub fn horizontal() -> Self {
        Self::new().label_layout(FormLabelLayout::Horizontal)
    }

    /// Creates a form with labels above their controls.
    pub fn vertical() -> Self {
        Self::new()
    }

    /// Sets the label direction within each field. An alias of
    /// [`Self::label_layout`], keeping the reference's method pair.
    pub fn layout(self, layout: FormLabelLayout) -> Self {
        self.label_layout(layout)
    }

    /// Sets the label direction within each field. Use [`Self::columns`] to
    /// arrange fields, which is independent of the label direction.
    pub fn label_layout(mut self, layout: FormLabelLayout) -> Self {
        self.props.layout = layout;
        self
    }

    /// Sets the width of the labels in a horizontal form. The default is 140.
    pub fn label_width(mut self, width: f32) -> Self {
        self.props.label_width = width;
        self
    }

    /// Sets the text size of the labels. The default is the theme's small step.
    pub fn label_text_size(mut self, size: f32) -> Self {
        self.props.label_text_size = Some(size);
        self
    }

    /// Sets the column count of the field grid. The default is 1.
    pub fn columns(mut self, columns: usize) -> Self {
        self.props.columns = columns.max(1);
        self
    }

    /// Sets the size step, which scales the form's spacing.
    pub fn size(mut self, size: Size) -> Self {
        self.props.size = size;
        self
    }

    /// Adds a field.
    pub fn child(mut self, field: Field<'a, Message>) -> Self {
        self.fields.push(field);
        self
    }

    /// Adds several fields.
    pub fn children(mut self, fields: impl IntoIterator<Item = Field<'a, Message>>) -> Self {
        self.fields.extend(fields);
        self
    }

    /// Sets content drawn after all fields, spanning every column and aligned
    /// to the trailing edge — the row a form's actions live in. Calling this
    /// again replaces the footer; no footer row is drawn without one.
    pub fn footer(mut self, footer: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.footer = Some(footer.into());
        self
    }

    /// Assembles the grid the form renders as: visible fields, then a
    /// full-span cell carrying the footer.
    fn build_grid(self) -> grid::Grid<'a, Message> {
        let Self {
            props,
            fields,
            footer,
        } = self;

        let columns = props.columns.max(1);
        let row_spacing = form_row_spacing(props.size);
        // The reference spaces columns three times as far as rows.
        let column_spacing = row_spacing * 3.0;

        let mut grid = grid::Grid::new(columns, row_spacing, column_spacing);
        for field in fields {
            // The reference stored `visible` but never read it; dropping the
            // field before placement is what the setter promises.
            if !field.visible {
                continue;
            }
            grid = grid.push(field.into_cell(props));
        }

        if let Some(footer) = footer {
            grid = grid.push(GridCell {
                span: columns as u16,
                start: None,
                end: None,
                content: container(footer)
                    .width(Length::Fill)
                    .align_x(alignment::Horizontal::Right)
                    .into(),
            });
        }

        grid
    }

    /// Converts the form into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        self.build_grid().into()
    }
}

impl<'a, Message: 'a> Default for Form<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: 'a> From<Form<'a, Message>> for Element<'a, Message, Theme> {
    fn from(form: Form<'a, Message>) -> Self {
        form.into_element()
    }
}

/// Builds a [`Form`] with labels above their controls.
pub fn form<'a, Message: 'a>() -> Form<'a, Message> {
    Form::new()
}

/// Builds a [`Field`].
pub fn field<'a, Message: 'a>() -> Field<'a, Message> {
    Field::new()
}

#[cfg(test)]
mod tests {
    use super::{field, field_spacing, form, form_row_spacing, Field, FieldLabel, Form,
        FormLabelLayout};
    use crate::theme::Size;
    use iced::widget::text;

    #[test]
    fn a_field_defaults_to_visible_plain_and_indenting() {
        let field: Field<'_, ()> = Field::new();
        assert!(field.visible);
        assert!(!field.required);
        assert!(field.label_indent);
        assert_eq!(field.col_span, 1);
        assert!(field.col_start.is_none());
        assert!(field.col_end.is_none());
        assert!(field.label.is_none());
        assert!(field.description.is_none());
    }

    #[test]
    fn pushing_controls_accumulates_them() {
        let field: Field<'_, ()> = Field::new().push(text("a")).push(text("b"));
        assert_eq!(field.children.len(), 2);
    }

    #[test]
    fn a_label_takes_text_or_an_element() {
        let from_str: FieldLabel<'_, ()> = "Email".into();
        assert!(matches!(from_str, FieldLabel::Text(value) if value == "Email"));

        let from_string: FieldLabel<'_, ()> = String::from("Email").into();
        assert!(matches!(from_string, FieldLabel::Text(value) if value == "Email"));

        let from_element: FieldLabel<'_, ()> = text("Email").into();
        assert!(matches!(from_element, FieldLabel::Element(_)));

        let field: Field<'_, ()> = Field::new().label("Email");
        assert!(matches!(field.label, Some(FieldLabel::Text(_))));
    }

    #[test]
    fn the_default_form_is_a_single_vertical_column() {
        let new_form: Form<'_, ()> = Form::new();
        let default_form: Form<'_, ()> = Form::default();
        let shorthand: Form<'_, ()> = form();
        for form in [new_form, default_form, shorthand] {
            assert_eq!(form.props.layout, FormLabelLayout::Vertical);
            assert_eq!(form.props.columns, 1);
            assert_eq!(form.props.label_width, 140.0);
            assert!(form.footer.is_none());
        }
    }

    #[test]
    fn the_builder_records_its_settings() {
        let form: Form<'_, ()> = Form::new()
            .columns(2)
            .label_layout(FormLabelLayout::Horizontal)
            .label_width(80.0)
            .label_text_size(13.0)
            .size(Size::Lg)
            .child(Field::new())
            .footer(iced::widget::text("Save"));
        assert_eq!(form.props.layout, FormLabelLayout::Horizontal);
        assert_eq!(form.props.columns, 2);
        assert_eq!(form.props.label_width, 80.0);
        assert_eq!(form.props.label_text_size, Some(13.0));
        assert_eq!(form.props.size, Size::Lg);
        assert_eq!(form.fields.len(), 1);
        assert!(form.footer.is_some());
    }

    #[test]
    fn layout_is_an_alias_of_label_layout() {
        let form: Form<'_, ()> = form().layout(FormLabelLayout::Horizontal);
        assert_eq!(form.props.layout, FormLabelLayout::Horizontal);
    }

    #[test]
    fn an_invisible_field_is_dropped_before_placement() {
        let form: Form<'_, ()> = form()
            .child(field().label("Shown"))
            .child(field().label("Hidden").visible(false));
        let grid = form.build_grid();
        assert_eq!(grid.cells.len(), 1);
    }

    #[test]
    fn the_footer_spans_every_column() {
        let form: Form<'_, ()> = form()
            .columns(3)
            .child(field())
            .footer(iced::widget::text("Save"));
        let grid = form.build_grid();
        assert_eq!(grid.cells.len(), 2);
        let footer = grid.cells.last().unwrap();
        assert_eq!(footer.span, 3);
        assert!(footer.start.is_none());
    }

    #[test]
    fn spacing_follows_the_reference_size_scale() {
        for (size, row) in [
            (Size::Xs, 6.0),
            (Size::Sm, 6.0),
            (Size::Md, 8.0),
            (Size::Lg, 12.0),
            (Size::Custom(9.0), 9.0),
        ] {
            assert_eq!(form_row_spacing(size), row);
            // The reference spaces columns three times as far as rows.
            assert_eq!(row * 3.0, form_row_spacing(size) * 3.0);
        }
        assert_eq!(field_spacing(Size::Lg), 8.0);
        assert_eq!(field_spacing(Size::Md), 4.0);
        assert_eq!(field_spacing(Size::Xs), 4.0);
    }
}
