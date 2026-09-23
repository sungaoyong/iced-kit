//! Modal dialogs and their composition parts.
//!
//! A dialog is a surface that asks the user something before the application
//! carries on. This module ports the `dialog/` component family of `gpui-kit`,
//! and it is built from two halves:
//!
//! - **Composition parts** — [`DialogHeader`], [`dialog_title`],
//!   [`dialog_description`], [`DialogContent`] and [`DialogFooter`]. These are
//!   the pieces a caller assembles when it lays out a dialog body itself.
//! - **Assembled dialogs** — [`Modal`], which blocks the page, [`Dialog`],
//!   which does not, and [`AlertDialog`], the blocking dialog with the
//!   opinionated defaults an interrupting message wants.
//!
//! # Where the state lives
//!
//! The reference opens a dialog by pushing it onto an imperative stack
//! (`Window::open_dialog`); its base layer owns the dialog's lifetime, focus
//! and key handling. iced has no such stack, so this crate keeps the split it
//! uses for every other overlay: the **application** holds whether a dialog is
//! open and passes it to [`overlay::layer`](crate::widgets::overlay::layer)
//! while it is.
//!
//! That is why these types have no `open`/`close`. It is also why the
//! reference's `keyboard` flag — Escape closes the dialog — has no counterpart
//! here: a keystroke reaches a widget through the application's `subscription`,
//! so dismissing on Escape stays the application's to wire. [`Modal::new`]
//! documents the pattern.
//!
//! # What was not ported
//!
//! The reference's `dispatch_anchor` is a `FocusHandle` that a dialog button
//! dispatches its `Confirm`/`Cancel` action from, so the action finds the
//! dialog rather than whatever holds focus. It exists because gpui routes an
//! action along the focus path, which a surface that keeps taking focus back —
//! a native web view, say — leaves empty. iced has no action routing: a button
//! carries its own `Message` to the application, so there is nothing for an
//! anchor to repair.
//!
//! # One deliberate difference
//!
//! The reference floats its close button over the surface at an absolute
//! position, inset 8px from the top-right corner. iced has no absolute
//! positioning, so the button is layered over the card in a
//! [`Stack`](iced::widget::Stack) and inset by [`close_inset`] on both axes,
//! which puts it in the same place.
//!
//! Two details there are load-bearing. The card's width is
//! [`Length::Fixed`](iced::Length::Fixed) rather than a filling width with a
//! maximum, because a stack adopts its first child's size hint: a filling card
//! makes the stack span the window and takes the dialog's centring with it. And
//! the button's inset is one value for both axes, not the surface's
//! vertical-only padding — a button 25px from the top and 9px from the right
//! reads as low rather than in the corner.

pub mod alert_dialog;
pub mod content;
pub mod description;
pub mod footer;
pub mod header;
pub mod modal;
pub mod title;

pub use alert_dialog::{AlertDialog, AlertTone};
pub use content::DialogContent;
pub use description::dialog_description;
pub use footer::{dialog_actions, dialog_close, DialogFooter};
pub use header::{header_with_icon, DialogHeader};
pub use modal::{Dialog, DialogButtonProps, DialogWidth, Modal};
pub use title::dialog_title;

/// The inset between a dialog's edge and its content.
///
/// The reference's `Edges::all(px(16.))`. A dialog is read all at once rather
/// than worked in, so it holds its content closer to the edge than a page does.
pub const DIALOG_PADDING: f32 = 16.0;

/// The gap between a dialog's sections — header, body, footer.
///
/// The reference derives this from the padding, `gap(paddings.top.max(px(8.)))`,
/// so a caller who pads the surface more also spaces its sections more. At the
/// default padding the two are equal.
pub const DIALOG_GAP: f32 = DIALOG_PADDING;

/// The gap between the lines of a dialog header.
///
/// Tighter than [`DIALOG_GAP`]: a title and its description are one thought, so
/// they sit closer to each other than to the body below them. The reference's
/// `gap_y_2`.
pub const HEADER_GAP: f32 = 8.0;

/// The gap between the buttons of a dialog footer.
pub const FOOTER_GAP: f32 = 8.0;

/// The font size of a dialog title.
///
/// The reference's `text_base`.
pub const TITLE_SIZE: f32 = 16.0;

/// The line height of a dialog title.
///
/// The reference sets `line_height(relative(1.25))`, so the line box follows
/// the size rather than a fixed pixel value.
pub const TITLE_LINE_HEIGHT: f32 = TITLE_SIZE * 1.25;

/// The font size of a dialog description.
///
/// The reference's `text_sm`.
pub const DESCRIPTION_SIZE: f32 = 14.0;

/// The line height of a dialog description.
pub const DESCRIPTION_LINE_HEIGHT: f32 = 20.0;

/// The inset between a floating control and the corner it sits in.
///
/// The single inset for both axes. The reference pulls its close button in from
/// the surface's padding by 10px and clamps that back up to 8,
/// `(paddings.top - px(10.)).max(px(8.))`; at the default padding both give the
/// same 8px, and the clamp is what stops a caller who reduces the padding from
/// pushing the button into the corner.
///
/// One inset for both axes is the point. The button is a small square in a
/// corner, so a reader judges its distance from the two edges against each
/// other: giving it the surface's vertical padding on top and nothing on the
/// right — which is what reusing [`surface_padding`] here does — leaves it
/// looking low and off the corner even though each margin is individually
/// reasonable.
#[must_use]
pub const fn close_inset(padding: f32) -> f32 {
    let pulled_in = padding - 10.0;

    if pulled_in > 8.0 {
        pulled_in
    } else {
        8.0
    }
}

/// The padding a floating control in a surface's corner is inset by.
///
/// Equal on every side, so the control sits the same distance from both edges
/// of the corner it occupies.
#[must_use]
pub fn corner_padding(padding: f32) -> iced::Padding {
    iced::Padding::new(0.0)
        .vertical(close_inset(padding))
        .horizontal(close_inset(padding))
}

/// The style every dialog surface is drawn with: a raised card.
///
/// Shared by [`Modal`], [`Dialog`] and [`AlertDialog`] so the three cannot
/// drift apart. They differ in behaviour and in what they put inside the
/// surface, not in how the surface looks.
///
/// # Why the width is `Fill` with a maximum rather than `Fixed`
///
/// A filling width resolves against the container's maximum, so the card is
/// exactly [`DialogWidth`](super::DialogWidth)'s width when the window can hold
/// it and shrinks to fit when it cannot. A fixed width would instead overflow a
/// narrow window, because nothing here can override it.
pub(crate) fn surface_class<'a>() -> iced::widget::container::StyleFn<'a, crate::theme::Theme> {
    Box::new(|theme: &crate::theme::Theme| {
        let colors = theme.colors();

        iced::widget::container::Style {
            background: Some(iced::Background::Color(colors.surface)),
            border: iced::Border {
                color: colors.border,
                width: 1.0,
                radius: f32::from(theme.radius().lg).into(),
            },
            shadow: super::floating_shadow(theme),
            text_color: Some(colors.surface_foreground),
            ..iced::widget::container::Style::default()
        }
    })
}

/// How far a dialog rises as it arrives, in logical pixels.
///
/// Short on purpose: a dialog appears at the centre of attention rather than
/// travelling there, so a long slide would read as it coming from somewhere.
pub(crate) const DIALOG_TRAVEL: f32 = 12.0;

/// The inset that pads a dialog section on its left and right only.
///
/// The reference gives the surface vertical padding and each section its own
/// horizontal padding (`px_0()` on the surface, `pl`/`pr` on the children).
/// The split matters for the body: a scrolling body's scrollbar then sits
/// against the surface's edge rather than inset from it.
#[must_use]
pub fn section_padding(padding: f32) -> iced::Padding {
    iced::Padding::new(0.0).horizontal(padding)
}

/// The inset that pads the surface above and below its content.
#[must_use]
pub fn surface_padding(padding: f32) -> iced::Padding {
    iced::Padding::new(0.0).vertical(padding)
}

#[cfg(test)]
mod tests {
    use super::{section_padding, surface_padding, DIALOG_GAP, DIALOG_PADDING, HEADER_GAP};

    /// The header gap is tighter than the section gap, so a title and its
    /// description read as one block rather than as two separate sections.
    const _HEADER_GAP_IS_TIGHTER: () = assert!(HEADER_GAP < DIALOG_GAP);

    #[test]
    fn a_section_is_padded_only_across_and_a_surface_only_down() {
        let section = section_padding(DIALOG_PADDING);
        assert_eq!(
            (section.left, section.right),
            (DIALOG_PADDING, DIALOG_PADDING)
        );
        assert_eq!((section.top, section.bottom), (0.0, 0.0));

        let surface = surface_padding(DIALOG_PADDING);
        assert_eq!(
            (surface.top, surface.bottom),
            (DIALOG_PADDING, DIALOG_PADDING)
        );
        assert_eq!((surface.left, surface.right), (0.0, 0.0));
    }
}
