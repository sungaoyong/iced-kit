//! A surface that claims the presses that land on it.
//!
//! iced routes a press through a [`Stack`](iced::widget::stack) from the
//! topmost layer down, and a layer stops it only by capturing the event. A
//! floating surface made of ordinary, non-interactive widgets — the blank half
//! of a drawer, a dialog's heading text — captures nothing, so a press on it
//! falls through to the layer beneath: the backdrop, which reads the press as
//! "dismiss me". A drawer that closes when its own blank half is clicked, and
//! reopens when the next click lands, is what that fall-through feels like.
//!
//! [`ClaimPress`] wraps a surface and captures every press that lands inside
//! it, the way a draggable dialog's surface does for its grab. Presses beside
//! the surface still reach the backdrop, which is exactly the dismissal
//! gesture.

use crate::theme::Theme;
use iced::advanced::widget::{tree, Operation, Widget};
use iced::advanced::{layout, mouse, overlay, Clipboard, Shell};
use iced::{Element, Event, Rectangle};

/// Wraps a floating surface so presses on it never reach the layer beneath.
///
/// A press a widget inside the surface has already claimed — a button, a text
/// field — is left alone; only the presses the surface's inert areas let fall
/// through are caught here.
#[must_use = "a ClaimPress does nothing unless it is turned into an Element"]
pub struct ClaimPress<'a, Message> {
    content: Element<'a, Message, Theme>,
}

impl<'a, Message: 'a> ClaimPress<'a, Message> {
    /// Wraps `content` so that a press on it is captured.
    pub fn new(content: impl Into<Element<'a, Message, Theme>>) -> Self {
        Self {
            content: content.into(),
        }
    }
}

impl<Message: Clone> Widget<Message, Theme, iced::Renderer> for ClaimPress<'_, Message> {
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

        // The surface claims the press only where nobody inside it has: a
        // button's press keeps its button, and everything else keeps the
        // surface closed against the backdrop beneath it.
        if is_press(event) && !shell.is_event_captured() && cursor.is_over(layout.bounds()) {
            shell.capture_event();
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

/// Whether the event is a press the surface should claim.
fn is_press(event: &Event) -> bool {
    matches!(
        event,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(iced::touch::Event::FingerPressed { .. })
    )
}

impl<'a, Message: Clone + 'a> From<ClaimPress<'a, Message>> for Element<'a, Message, Theme> {
    fn from(claim: ClaimPress<'a, Message>) -> Self {
        Element::new(claim)
    }
}

#[cfg(test)]
mod tests {
    use super::ClaimPress;
    use crate::theme::Theme;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Pressed,
    }

    /// The wrapper hands its own size over, so a claimed surface keeps the
    /// box it would have had.
    #[test]
    fn a_claimed_surface_takes_its_content_size() {
        use iced::advanced::Widget;
        use iced::Length;

        let claim: ClaimPress<'_, Message> =
            ClaimPress::new(crate::widgets::button("Save").width(Length::Fill));

        assert_eq!(Widget::size(&claim).width, Length::Fill);
    }

    #[test]
    fn a_claimed_surface_renders() {
        let element: iced::Element<'_, Message, Theme> =
            ClaimPress::new(crate::widgets::button("Save").on_press(Message::Pressed)).into();

        drop(element);
    }
}
