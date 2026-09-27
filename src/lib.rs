//! A shadcn/ui-flavored component library and design system for
//! [iced](https://iced.rs/), porting the visual language of
//! [gpui-kit](https://github.com/longbridge/gpui-kit).
//!
//! # Design
//!
//! iced 0.14 widgets are generic over their theme type, so this crate ships its
//! own [`Theme`] carrying a full set of semantic design tokens. Implementing the
//! upstream widget `Catalog` traits for that theme means stock iced widgets and
//! `iced-kit` components render with one consistent look:
//!
//! ```no_run
//! use iced_kit::Theme;
//!
//! # #[derive(Debug, Clone)] enum Message {}
//! # fn main() -> iced::Result {
//! #     fn update(_: &mut (), _: Message) {}
//! #     fn view(_: &()) -> iced::Element<'_, Message, Theme> {
//! #         iced::widget::text("hi").into()
//! #     }
//! iced::application(|| (), update, view)
//!     .theme(|_: &()| Theme::dark())
//!     .run()
//! # }
//! ```
//!
//! # Getting started
//!
//! ```
//! use iced_kit::prelude::*;
//! ```

// Docking layout, ported from `iced_dock` (MIT). See `NOTICE`.
#[cfg(feature = "dock")]
pub mod dock;

#[cfg(feature = "i18n")]
pub mod i18n;
pub mod icons;
pub mod motion;
pub mod setting;
pub mod theme;
pub mod widgets;

#[cfg(feature = "i18n")]
pub use i18n::I18n;
pub use theme::{Colors, Motion, Size, Theme, Tokens};

/// The imports most applications want.
pub mod prelude {
    #[cfg(feature = "i18n")]
    pub use crate::i18n::I18n;
    pub use crate::icons::IconName;
    pub use crate::theme::{Size, Theme};
    pub use crate::widgets::*;
}
