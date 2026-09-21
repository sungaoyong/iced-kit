//! Text inputs.
//!
//! A field's border belongs to the frame around it, not to the control inside
//! it. That is what lets a prefix, a suffix, a clear button and a spinner share
//! one box, and it is why every field in this crate — the single-line input, the
//! text area, the numeric spin button and the one-time-code row — draws the same
//! border even though the controls underneath them differ.
//!
//! # Reading focus
//!
//! A widget cannot ask iced "is my child focused?" through the public API, so
//! the frame reads the control's own state out of the widget tree — the same
//! downcast `iced_widget`'s own `combo_box` performs to decide whether to show
//! its selection.
//!
//! # What iced does not offer
//!
//! Three things `gpui-kit` has no direct equivalent for here:
//!
//! - **Read-only.** iced treats a text input with no handler as disabled, and
//!   has no state in between. [`TextInput::readonly`] therefore withholds the
//!   handler, which keeps the field's appearance and its value readable but
//!   costs it the caret.
//! - **Accessibility.** iced 0.14 has no accessibility tree, so the labels,
//!   roles and content types `gpui-kit` forwards to the platform have nowhere
//!   to go. [`TextInput::label`] draws a visible label instead.
//! - **Per-part styling.** iced's `text_input` class can see only the control's
//!   own status, not that the caller marked it invalid or that a group owns the
//!   frame. The frame resolves those itself and passes the result down.

mod frame;
mod group;
mod text_input;

pub use group::{
    addon, group_button, group_icon_button, input_group, AddonAlignment, GroupControl, InputGroup,
    InputGroupAddon,
};
pub use text_input::{label, password, text_area, text_input, TextArea, TextInput};

pub(crate) use frame::{ControlKind, FieldFrame};
