//! A trigger that reports where it sits.
//!
//! The split components — the combobox, the date picker, the colour picker —
//! deliberately report intent without a position: [`on_toggle`] carries a
//! message and nothing else, and the application hosts the panel itself
//! through [`Layer`]. Hosting it means anchoring it, and anchoring it means
//! knowing where the trigger is — which iced has no API for. A button's press
//! carries no cursor point, and no widget is asked for its window-space
//! bounds outside of layout.
//!
//! [`Trigger`] is that missing half of the handshake. It wraps a trigger,
//! forwards everything to it untouched, and the moment a press lands inside
//! its bounds it publishes the wrapper's window-space [`Rectangle`] — before
//! the content sees the event, so the anchor arrives ahead of the toggle
//! message the content publishes, and the application never anchors a panel
//! from a stale press.
//!
//! ```
//! # use iced_kit::widgets::overlay::{layer, trigger, Layer};
//! # use iced_kit::{widgets::button, Theme};
//! # #[derive(Clone, Debug)] enum Message { Anchor(iced::Rectangle), Toggle }
//! # fn view() -> iced::Element<'static, Message, Theme> {
//! let combobox = button("Country").on_press(Message::Toggle);
//!
//! // The application records the trigger's bounds when pressed, and hosts
//! // the panel anchored to them.
//! layer(
//!     trigger(combobox, Message::Anchor),
//!     Layer::default(),
//! )
//! # }
//! ```
//!
//! [`on_toggle`]: crate::widgets::combobox::ComboBox::on_toggle
//! [`Layer`]: crate::widgets::overlay::Layer

use crate::theme::Theme;
use iced::advanced::widget::{tree, Operation, Widget};
use iced::advanced::{layout, mouse, overlay, Clipboard, Shell};
use iced::{Element, Event, Rectangle};

/// Wraps a trigger so the application can learn where it sits.
///
/// The bounds are published the instant a press lands within them, before the
/// content processes the event. A press outside the bounds is forwarded
/// untouched and reports nothing.
///
/// # Why the wrapper rather than a position-aware press
///
/// iced's [`button`](iced::widget::button) reports a press as a message alone,
/// and the position-aware events its [`MouseArea`](iced::widget::MouseArea)
/// offers are cursor points, not widget bounds. A panel anchored to the
/// cursor sits where the hand happened to be; one anchored to the trigger's
/// rectangle sits below the control that owns it, which is what the split
/// components promise.
#[must_use = "a Trigger does nothing unless it is turned into an Element"]
pub struct Trigger<'a, Message> {
    content: Element<'a, Message, Theme>,
    on_press: Box<dyn Fn(Rectangle) -> Message + 'a>,
}

impl<'a, Message: 'a> Trigger<'a, Message> {
    /// Wraps `content` so that a press on it publishes its bounds.
    pub fn new(
        content: impl Into<Element<'a, Message, Theme>>,
        on_press: impl Fn(Rectangle) -> Message + 'a,
    ) -> Self {
        Self {
            content: content.into(),
            on_press: Box::new(on_press),
        }
    }
}

/// Wraps a trigger so that a press on it publishes its bounds.
///
/// A shorthand for [`Trigger::new`], reading the way the split components'
/// hosting story reads: the trigger reports intent, and this tells the
/// application where to put the panel that intent opens.
pub fn trigger<'a, Message: 'a>(
    content: impl Into<Element<'a, Message, Theme>>,
    on_press: impl Fn(Rectangle) -> Message + 'a,
) -> Trigger<'a, Message> {
    Trigger::new(content, on_press)
}

impl<Message: Clone> Widget<Message, Theme, iced::Renderer> for Trigger<'_, Message> {
    fn children(&self) -> Vec<tree::Tree> {
        vec![tree::Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut tree::Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> iced::Size<iced::Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut tree::Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut tree::Tree,
        layout: layout::Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut tree::Tree,
        event: &Event,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        // The anchor precedes whatever the content publishes, so a toggle that
        // opens a panel reads the bounds of the press that opened it.
        if is_press(event) && cursor.is_over(layout.bounds()) {
            let bounds = layout.bounds();
            shell.publish((self.on_press)(bounds));
        }

        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &tree::Tree,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
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
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut tree::Tree,
        layout: layout::Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: iced::Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, iced::Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

/// Whether the event is a press the anchor should be reported on.
///
/// A context menu opens on a right press like a panel opens on a left one, so
/// both report; the wrapper does not decide what the application does with
/// either.
fn is_press(event: &Event) -> bool {
    matches!(
        event,
        Event::Mouse(mouse::Event::ButtonPressed(
            mouse::Button::Left | mouse::Button::Right
        )) | Event::Touch(iced::touch::Event::FingerPressed { .. })
    )
}

impl<'a, Message: Clone + 'a> From<Trigger<'a, Message>> for Element<'a, Message, Theme> {
    fn from(trigger: Trigger<'a, Message>) -> Self {
        Element::new(trigger)
    }
}

#[cfg(test)]
mod tests {
    use super::{trigger, Trigger};
    use crate::theme::Theme;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Anchored(iced::Rectangle),
        Toggled,
    }

    #[test]
    fn a_trigger_renders() {
        let element: iced::Element<'_, Message, Theme> = trigger(
            crate::widgets::button("Open").on_press(Message::Toggled),
            Message::Anchored,
        )
        .into();

        drop(element);
    }

    /// The wrapper hands its own size over to whoever lays it out, so a
    /// wrapped trigger fills its row exactly as an unwrapped one would.
    #[test]
    fn a_trigger_takes_its_content_size() {
        use iced::advanced::Widget;
        use iced::Length;

        let filling: Trigger<'_, Message> = Trigger::new(
            crate::widgets::button("Open").width(Length::Fill),
            Message::Anchored,
        );
        assert_eq!(Widget::size(&filling).width, Length::Fill);

        let hugging: Trigger<'_, Message> =
            Trigger::new(crate::widgets::button("Open"), Message::Anchored);
        assert_ne!(Widget::size(&hugging).width, Length::Fill);
    }
}
