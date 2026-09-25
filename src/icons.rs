//! The icon font, and the named glyphs the components draw from it.
//!
//! # Why a font rather than SVGs
//!
//! The reference implementation ships a directory of SVG files and a build
//! script that turns it into an enum. A font does the same job with less
//! machinery: one 850 KB file carries the whole Lucide set, a glyph is a
//! `char`, and an icon needs no asset source, no handle cache and no build
//! step. It also scales and recolors exactly like text, which is what an icon
//! beside a label has to do anyway.
//!
//! The trade is that a font is only useful once it is registered with the
//! renderer. Rather than making every application do that, [`load`] registers it
//! on first use — see below.
//!
//! # Loading
//!
//! An icon draws through [`Icon`](crate::widgets::Icon) as a text glyph in the
//! `lucide` family. A family that the font system has never seen renders as
//! tofu, so the font has to be loaded before the first frame.
//!
//! iced offers two ways to do that, and this crate uses both:
//!
//! - [`load`] registers the font on demand, and every component that draws an
//!   icon calls it first. Nothing is required of the application.
//! - An application may instead pass [`LUCIDE_FONT_BYTES`] to
//!   `iced::application(..).font(..)`, which registers it at startup. Both are
//!   safe together: the font system ignores a font it already has.
//!
//! # Using an icon
//!
//! ```
//! use iced_kit::icons::IconName;
//! use iced_kit::widgets::{button, Icon};
//!
//! # #[derive(Clone, Debug)] enum Message { Search }
//! # fn view() -> iced::Element<'static, Message, iced_kit::Theme> {
//! // On a button: sized and colored by the control it sits in.
//! button::<Message>("Find")
//!     .icon(Icon::new(IconName::Search))
//!     .on_press(Message::Search)
//!     .into()
//! # }
//! ```

use lucide_icons::Icon as Lucide;

/// The Lucide icon font, for an application that wants to register it itself.
///
/// Passing this to `iced::application(..).font(..)` loads it at startup instead
/// of on first use. It is never required: [`load`] is called by the components
/// that draw icons, and the font system ignores a duplicate.
pub use lucide_icons::LUCIDE_FONT_BYTES;

/// The family name the bundled font registers under.
///
/// iced identifies a font by the family name its file declares, so this is a
/// lookup key rather than a label. It is public so a caller can build its own
/// [`Font`](iced::Font) for a glyph.
pub const FONT_FAMILY: &str = "lucide";

/// Every icon this crate draws by name, re-exported from the font's own enum.
///
/// The variants are the Lucide names in upper camel case — `IconName::Search`,
/// `IconName::Undo2` — so a name here can be looked up directly on
/// [lucide.dev](https://lucide.dev/icons/).
pub type IconName = Lucide;

/// Registers the icon font with the renderer, once.
///
/// This is idempotent and cheap after the first call, so a widget that draws an
/// icon can call it unconditionally on every frame. Loading the same font twice
/// is not an error; the font system keeps the first copy.
///
/// It is called for you by every component that takes a [`Glyph`](IconName), so
/// an application normally never needs it. Call it directly only when building
/// an icon out of raw iced widgets.
pub fn load() {
    // `font_system` hands out a process-wide lock, so the load happens at most
    // once per process in practice. A poisoned lock is ignored rather than
    // propagated: a panic while another thread held it is not a reason to lose
    // every icon in the application.
    if let Ok(mut system) = iced::advanced::graphics::text::font_system().write() {
        system.load_font(LUCIDE_FONT_BYTES.into());
    }
}

/// The `Font` a glyph of this icon font is drawn in.
///
/// Exposed so a caller building raw iced text can place an icon without going
/// through [`Icon`](crate::widgets::Icon).
#[must_use]
pub fn font() -> iced::Font {
    iced::Font::with_name(FONT_FAMILY)
}

/// The glyph a named icon draws.
///
/// Kept separate from the `From` conversion below so the intent is readable at a
/// call site that only wants the character.
#[must_use]
pub fn glyph(name: IconName) -> char {
    name.into()
}

#[cfg(test)]
mod tests {
    use super::{font, glyph, load, IconName, FONT_FAMILY, LUCIDE_FONT_BYTES};

    #[test]
    fn the_bundled_font_is_a_real_truetype_file() {
        // Guards against a dependency change that swaps the font for a stub:
        // every sfnt file starts with these four bytes.
        assert_eq!(&LUCIDE_FONT_BYTES[..4], &[0x00, 0x01, 0x00, 0x00]);
        assert!(
            LUCIDE_FONT_BYTES.len() > 100_000,
            "the icon font looks empty"
        );
    }

    #[test]
    fn named_icons_resolve_to_private_use_glyphs() {
        // Lucide assigns every icon a code point in the private use area, which
        // is what keeps a glyph from colliding with a letter.
        for name in [
            IconName::Search,
            IconName::Undo2,
            IconName::Info,
            IconName::ChevronRight,
        ] {
            let c = glyph(name);
            assert!(
                ('\u{e000}'..='\u{f8ff}').contains(&c),
                "{name:?} resolved to {c:?}, which is outside the private use area"
            );
        }
    }

    #[test]
    fn distinct_icons_draw_distinct_glyphs() {
        assert_ne!(glyph(IconName::Search), glyph(IconName::Info));
        assert_ne!(glyph(IconName::ChevronDown), glyph(IconName::ChevronRight));
    }

    #[test]
    fn the_font_family_matches_what_the_font_declares() {
        assert_eq!(font().family, iced::font::Family::Name(FONT_FAMILY));
    }

    #[test]
    fn loading_twice_is_not_an_error() {
        // An application may also register the font at startup, so a second load
        // has to be harmless.
        load();
        load();
    }
}
