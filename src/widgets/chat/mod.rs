//! The chat family: messages, bubbles, markers, attachments, and a
//! tail-following transcript scroller.
//!
//! Ported in spirit from `gpui-kit`'s chat components, adapted to this crate's
//! `iced` idiom: builder structs that collect children into a `Vec<Element>` and
//! resolve their look from the semantic tokens on [`crate::Theme`].
//!
//! See `docs/superpowers/specs/2026-09-24-chat-family-design.md`.

pub mod attachment;
pub mod bubble;
pub mod marker;
pub mod message;
pub mod scroller;

// Re-exports are switched on as each component lands (Tasks 1-6 of the plan).
// pub use attachment::{ ... };
// pub use bubble::{ ... };
pub use marker::{marker, Marker, MarkerLoadingStyle, MarkerVariant};
// pub use message::{ ... };
// pub use scroller::{ ... };
