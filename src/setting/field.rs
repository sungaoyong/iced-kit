//! Setting fields: the typed control bound to one value.
//!
//! A field is the part of a setting item the user actually manipulates. The
//! reference resolves one by erasing a `Fn(&App) -> T` getter and a
//! `Fn(T, &mut App)` setter and downcasting the type at render time; iced has no
//! global app state to read from, and its `Element` is already type-erased, so
//! neither the downcast nor the erasure is needed here. A field is built from
//! the value and a message constructor, and it renders itself.
//!
//! # Why the handlers are `Clone`
//!
//! A field is drawn on every frame, so its render closure is an `Fn` that can be
//! called repeatedly — and each call has to hand the underlying iced widget a
//! fresh handler, because constructing a widget consumes it. Requiring `Clone`
//! on the handler is what makes that possible; a plain `Fn` plus `Clone` is the
//! cheapest bound that allows it, and every `Fn` closure over `Copy` data
//! satisfies it.

use crate::theme::{Size, Theme};
use crate::widgets::{muted_text, select, switch};
use iced::{Element, Length};

/// The control a [`SettingField`] draws.
///
/// This exists so a panel can reason about a field without knowing its value
/// type — chiefly to decide how wide the control column should be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingFieldKind {
    /// A sliding on/off switch.
    Switch,
    /// A labeled checkbox.
    Checkbox,
    /// A single-line text field.
    Text,
    /// A numeric stepper.
    Number,
    /// A dropdown of named choices.
    Select,
    /// A caller-supplied element.
    Custom,
}

impl SettingFieldKind {
    /// Whether this control wants a narrow column rather than the full width of
    /// the field area.
    ///
    /// A switch or a checkbox is a small control, so giving it the whole column
    /// would strand it far from the setting it belongs to. A text field or a
    /// dropdown reads as a field and takes the width.
    #[must_use]
    pub fn prefers_narrow_column(self) -> bool {
        matches!(self, Self::Switch | Self::Checkbox)
    }
}

/// The value a field was built from.
///
/// A panel never needs this — it holds the value itself — but keeping the value
/// alongside the kind makes a field self-describing, which is what lets a dirty
/// check and a reset work without the caller repeating the value.
#[derive(Debug, Clone, PartialEq)]
pub enum SettingValue {
    /// A boolean.
    Bool(bool),
    /// A number.
    Number(f64),
    /// A string.
    Text(String),
    /// A dropdown's selected value, if one is selected.
    Choice(Option<String>),
    /// A value this crate cannot describe, from a custom element.
    Custom,
}

/// One control in a setting item, bound to a value and a message.
///
/// Built with a constructor rather than a struct literal, because each control
/// needs a different closure shape and the constructors are what hide that.
///
/// ```
/// # use iced_kit::setting::SettingField;
/// # #[derive(Clone, Debug)] enum Message { Enabled(bool), Name(String) }
/// // A switch reports the new value.
/// let s = SettingField::switch(true, Message::Enabled);
/// // A text field reports the new value.
/// let t = SettingField::text("Ada", Message::Name);
/// # let _ = (s, t);
/// ```
#[must_use = "a SettingField does nothing unless it is given to a SettingItem"]
pub struct SettingField<'a, Message> {
    kind: SettingFieldKind,
    value: SettingValue,
    /// The default a reset restores, when the caller declared one.
    default: Option<SettingValue>,
    render: Box<dyn Fn(bool, Size) -> Element<'a, Message, Theme> + 'a>,
}

/// Inspecting a field needs nothing of its message type.
///
/// These are separated from the constructors because a panel asks a field
/// whether it is dirty — to decide if a reset control belongs on the page —
/// without ever knowing what message the field reports. Keeping them here means
/// that question does not drag a `Clone` bound through every caller.
impl<Message> SettingField<'_, Message> {
    /// The control this field draws.
    #[must_use]
    pub fn kind(&self) -> SettingFieldKind {
        self.kind
    }

    /// The value this field was built from.
    #[must_use]
    pub fn value(&self) -> &SettingValue {
        &self.value
    }

    /// The default this field resets to, if one was declared.
    #[must_use]
    pub fn default(&self) -> Option<&SettingValue> {
        self.default.as_ref()
    }

    /// Whether this field differs from its declared default.
    ///
    /// A field with no default is never dirty: there is nothing to reset it to,
    /// so reporting a change would offer a reset that does nothing.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        match &self.default {
            Some(default) => !values_match(default, &self.value),
            None => false,
        }
    }

    /// Whether a reset would change anything.
    #[must_use]
    pub fn is_resettable(&self) -> bool {
        self.default.is_some()
    }
}

impl<'a, Message: Clone + 'a> SettingField<'a, Message> {
    /// Declares the value a reset restores.
    ///
    /// A field without one is not resettable, and a group whose every field is
    /// unresettable shows no reset control — which is what keeps one from
    /// appearing over settings that have nothing to reset to.
    pub fn default_value(mut self, value: impl Into<SettingValue>) -> Self {
        self.default = Some(value.into());
        self
    }

    /// Draws the control.
    ///
    /// `disabled` comes from the item or group rather than from the field, so
    /// that disabling a group reaches every field inside it without the caller
    /// repeating the flag on each one.
    pub(crate) fn render(&self, disabled: bool, size: Size) -> Element<'a, Message, Theme> {
        (self.render)(disabled, size)
    }

    /// A sliding on/off switch.
    ///
    /// The switch carries no label of its own, because a settings row puts the
    /// label in its own column where it can wrap and sit above a description.
    pub fn switch(value: bool, on_change: impl Fn(bool) -> Message + Clone + 'a) -> Self {
        Self {
            kind: SettingFieldKind::Switch,
            value: SettingValue::Bool(value),
            default: None,
            render: Box::new(move |disabled, _size| {
                if disabled {
                    // A disabled switch is drawn with no handler, which is what
                    // iced reads as inert.
                    return muted_text(if value { "On" } else { "Off" }).into();
                }

                switch("", value, on_change.clone())
            }),
        }
    }

    /// A checkbox with no label of its own.
    pub fn checkbox(value: bool, on_change: impl Fn(bool) -> Message + Clone + 'a) -> Self {
        Self {
            kind: SettingFieldKind::Checkbox,
            value: SettingValue::Bool(value),
            default: None,
            render: Box::new(move |disabled, _size| {
                if disabled {
                    return muted_text(if value { "Checked" } else { "Unchecked" }).into();
                }

                crate::widgets::checkbox("", value, on_change.clone())
            }),
        }
    }

    /// A single-line text field.
    pub fn text(
        value: impl Into<String>,
        on_change: impl Fn(String) -> Message + Clone + 'a,
    ) -> Self {
        let value = value.into();

        Self {
            kind: SettingFieldKind::Text,
            value: SettingValue::Text(value.clone()),
            default: None,
            render: Box::new(move |disabled, size| {
                let field = crate::widgets::text_input::<Message>("", &value)
                    .size(size)
                    .width(Length::Fill);

                if disabled {
                    field.disabled(true).into()
                } else {
                    field.on_input(on_change.clone()).into()
                }
            }),
        }
    }

    /// A numeric stepper.
    ///
    /// `range` is the inclusive range the value is clamped into, and the
    /// steppers step by one.
    pub fn number(
        value: f64,
        range: std::ops::RangeInclusive<f64>,
        on_change: impl Fn(f64) -> Message + Clone + 'a,
    ) -> Self {
        Self {
            kind: SettingFieldKind::Number,
            value: SettingValue::Number(value),
            default: None,
            render: Box::new(move |disabled, size| {
                crate::widgets::number_input::<Message>("", value, range.clone(), on_change.clone())
                    .no_label()
                    .size(size)
                    .width(Length::Fill)
                    .disabled(disabled)
                    .into()
            }),
        }
    }

    /// A dropdown of named choices.
    ///
    /// `options` pairs each value with the label shown for it, and `selected` is
    /// the currently chosen value. The label is what the closed control shows;
    /// the value is what `on_change` receives.
    ///
    /// ```
    /// # use iced_kit::setting::SettingField;
    /// # #[derive(Clone, Debug)] enum Message { Density(String) }
    /// SettingField::select(
    ///     vec![("compact".to_owned(), "Compact".to_owned())],
    ///     Some("compact".to_owned()),
    ///     Message::Density,
    /// );
    /// ```
    pub fn select(
        options: Vec<(String, String)>,
        selected: Option<String>,
        on_change: impl Fn(String) -> Message + Clone + 'a,
    ) -> Self {
        Self {
            kind: SettingFieldKind::Select,
            value: SettingValue::Choice(selected.clone()),
            default: None,
            render: Box::new(move |disabled, size| {
                let labels: Vec<String> = options.iter().map(|(_, label)| label.clone()).collect();
                let keys: Vec<String> = options.iter().map(|(key, _)| key.clone()).collect();

                let current_label = selected.as_ref().and_then(|current| {
                    options
                        .iter()
                        .position(|(key, _)| key == current)
                        .and_then(|ix| labels.get(ix).cloned())
                });

                if disabled {
                    // iced's pick list is disabled by withholding the handler,
                    // the same lever its own `text_input` uses. The message is
                    // never constructed, so any value will do.
                    return select(labels, current_label, |_| unreachable!())
                        .width(Length::Fill)
                        .text_size(size.text().size)
                        .into();
                }

                let labels_for_pick = labels.clone();
                let keys_for_pick = keys.clone();
                let on_change = on_change.clone();

                select(labels, current_label, move |picked: String| {
                    // The pick list reports the label it drew, so the label is
                    // mapped back to the value the caller asked for.
                    let ix = labels_for_pick
                        .iter()
                        .position(|label| *label == picked)
                        .unwrap_or(0);
                    let key = keys_for_pick.get(ix).cloned().unwrap_or_default();

                    on_change(key)
                })
                .width(Length::Fill)
                .text_size(size.text().size)
                .into()
            }),
        }
    }

    /// A caller-supplied element.
    ///
    /// The renderer is told whether the field is disabled, so a custom control
    /// can honor it. iced has no generic way to make an arbitrary element inert,
    /// so unlike the built-in kinds this cannot do it for you.
    pub fn custom(render: impl Fn(bool, Size) -> Element<'a, Message, Theme> + 'a) -> Self {
        Self {
            kind: SettingFieldKind::Custom,
            value: SettingValue::Custom,
            default: None,
            render: Box::new(render),
        }
    }
}

/// Whether two stored values are the same, for dirty checking.
///
/// Numbers compare by value rather than by bit pattern, so a value that
/// round-trips through text — `1.0` formatted and reparsed — is not reported as
/// changed, which would otherwise make every numeric setting offer a reset the
/// moment it was first drawn.
fn values_match(a: &SettingValue, b: &SettingValue) -> bool {
    match (a, b) {
        (SettingValue::Bool(a), SettingValue::Bool(b)) => a == b,
        (SettingValue::Number(a), SettingValue::Number(b)) => (a - b).abs() < f64::EPSILON,
        (SettingValue::Text(a), SettingValue::Text(b)) => a == b,
        (SettingValue::Choice(a), SettingValue::Choice(b)) => a == b,
        (SettingValue::Custom, SettingValue::Custom) => true,
        _ => false,
    }
}

impl From<bool> for SettingValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<f64> for SettingValue {
    fn from(value: f64) -> Self {
        Self::Number(value)
    }
}

impl From<String> for SettingValue {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<&str> for SettingValue {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

/// A zero-width spacer, for a field area with nothing in it.
pub(crate) fn empty_field<'a, Message: 'a>() -> Element<'a, Message, Theme> {
    iced::widget::Space::new().width(Length::Fill).into()
}

#[cfg(test)]
mod tests {
    use super::{SettingField, SettingFieldKind, SettingValue};

    #[derive(Debug, Clone, PartialEq)]
    enum Msg {
        Bool(bool),
        Text(String),
        Num(f64),
        Choice(String),
    }

    #[test]
    fn each_constructor_reports_its_own_kind() {
        assert_eq!(
            SettingField::switch(true, Msg::Bool).kind(),
            SettingFieldKind::Switch
        );
        assert_eq!(
            SettingField::checkbox(false, Msg::Bool).kind(),
            SettingFieldKind::Checkbox
        );
        assert_eq!(
            SettingField::text("a", Msg::Text).kind(),
            SettingFieldKind::Text
        );
        assert_eq!(
            SettingField::number(1.0, 0.0..=2.0, Msg::Num).kind(),
            SettingFieldKind::Number
        );
        assert_eq!(
            SettingField::select(vec![], None, Msg::Choice).kind(),
            SettingFieldKind::Select
        );
        assert_eq!(
            SettingField::<Msg>::custom(|_, _| iced::widget::text("x").into()).kind(),
            SettingFieldKind::Custom
        );
    }

    #[test]
    fn a_field_remembers_the_value_it_was_built_from() {
        assert_eq!(
            SettingField::switch(true, Msg::Bool).value(),
            &SettingValue::Bool(true)
        );
        assert_eq!(
            SettingField::text("Ada", Msg::Text).value(),
            &SettingValue::Text("Ada".to_owned())
        );
        assert_eq!(
            SettingField::number(2.5, 0.0..=10.0, Msg::Num).value(),
            &SettingValue::Number(2.5)
        );
        assert_eq!(
            SettingField::select(vec![], Some("a".to_owned()), Msg::Choice).value(),
            &SettingValue::Choice(Some("a".to_owned()))
        );
    }

    #[test]
    fn a_field_with_no_default_is_never_dirty_and_never_resettable() {
        let field = SettingField::switch(true, Msg::Bool);

        assert!(
            !field.is_resettable(),
            "no default means nothing to reset to"
        );
        assert!(
            !field.is_dirty(),
            "a field with nothing to reset to must not offer a reset"
        );
    }

    #[test]
    fn a_field_that_matches_its_default_is_clean() {
        let field = SettingField::switch(true, Msg::Bool).default_value(true);

        assert!(field.is_resettable());
        assert!(!field.is_dirty());
    }

    #[test]
    fn a_field_that_differs_from_its_default_is_dirty() {
        let field = SettingField::switch(false, Msg::Bool).default_value(true);

        assert!(field.is_resettable());
        assert!(field.is_dirty(), "a changed value must offer a reset");
    }

    #[test]
    fn dirty_checking_compares_numbers_by_value() {
        // A whole number that round-trips through a text field must not read as
        // changed, or every numeric setting would offer a reset immediately.
        let field = SettingField::number(1.0, 0.0..=10.0, Msg::Num).default_value(1.0);
        assert!(!field.is_dirty());
    }

    #[test]
    fn dirty_checking_notices_a_number_that_moved() {
        let field = SettingField::number(2.0, 0.0..=10.0, Msg::Num).default_value(1.0);
        assert!(field.is_dirty());
    }

    #[test]
    fn a_default_of_a_different_kind_does_not_match() {
        // Guards the `_ => false` arm: comparing a bool against a number has to
        // report a difference rather than panicking or silently matching.
        let field = SettingField::switch(true, Msg::Bool).default_value(1.0);
        assert!(field.is_dirty());
    }

    #[test]
    fn dirty_checking_compares_text_by_content() {
        let same = SettingField::text("Ada", Msg::Text).default_value("Ada");
        assert!(!same.is_dirty());

        let changed = SettingField::text("Grace", Msg::Text).default_value("Ada");
        assert!(changed.is_dirty());
    }

    #[test]
    fn dirty_checking_compares_a_choice_by_its_value() {
        let same = SettingField::select(vec![], Some("dark".to_owned()), Msg::Choice)
            .default_value(SettingValue::Choice(Some("dark".to_owned())));
        assert!(!same.is_dirty());

        let changed = SettingField::select(vec![], Some("light".to_owned()), Msg::Choice)
            .default_value(SettingValue::Choice(Some("dark".to_owned())));
        assert!(changed.is_dirty());
    }

    #[test]
    fn a_custom_field_is_never_dirty_on_its_own() {
        // A custom element owns its own state, so the panel cannot tell whether
        // it changed; only a declared default can make it resettable.
        let field = SettingField::<Msg>::custom(|_, _| iced::widget::text("x").into());
        assert!(!field.is_dirty());
        assert!(!field.is_resettable());
    }

    #[test]
    fn narrow_columns_are_reserved_for_the_boolean_controls() {
        assert!(SettingFieldKind::Switch.prefers_narrow_column());
        assert!(SettingFieldKind::Checkbox.prefers_narrow_column());

        for kind in [
            SettingFieldKind::Text,
            SettingFieldKind::Number,
            SettingFieldKind::Select,
            SettingFieldKind::Custom,
        ] {
            assert!(
                !kind.prefers_narrow_column(),
                "{kind:?} reads as a field and should take the width"
            );
        }
    }

    #[test]
    fn values_convert_from_their_natural_rust_types() {
        assert_eq!(SettingValue::from(true), SettingValue::Bool(true));
        assert_eq!(SettingValue::from(1.5), SettingValue::Number(1.5));
        assert_eq!(SettingValue::from("a"), SettingValue::Text("a".to_owned()));
        assert_eq!(
            SettingValue::from("a".to_owned()),
            SettingValue::Text("a".to_owned())
        );
    }
}
