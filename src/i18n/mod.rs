//! Internationalization support for iced-kit applications.
//!
//! This module provides a lightweight, synchronous translation layer that
//! integrates with iced's widget system. Applications hold an [`I18n`]
//! context and use it to resolve translated strings, which are then passed
//! to any widget constructor that accepts `impl text::IntoFragment`.
//!
//! Two backends are available:
//!
//! - **Core** (`i18n` feature): the [`Translator`] trait and [`I18n`]
//!   context, with no backend dependencies. Implement the trait with your
//!   own key-value store, or use the macro [`tr!`](crate::tr!) to resolve
//!   keys against an `i18n` variable in scope.
//! - **Fluent** (`i18n-fluent` feature): [`FluentTranslator`], a ready-made
//!   backend that loads `.ftl` files and supports ICU `MessageFormat`,
//!   plurals, and gendered messages.
//!
//! # Quick start
//!
//! ```no_run
//! use iced_kit::i18n::{FluentTranslator, I18n};
//!
//! // Load English translations
//! let translator = FluentTranslator::from_str("en", r#"
//! -save = Save
//! -cancel = Cancel
//! "#).expect("valid ftl");
//!
//! let i18n = I18n::new(translator);
//!
//! // In your view:
//! // button(i18n.tr("save"))  → button("Save")
//! // or, with a local `i18n` binding:
//! // button(tr!("save"))      → button("Save")
//! ```

mod tr;

#[cfg(feature = "i18n-fluent")]
mod fluent;

#[cfg(feature = "i18n-fluent")]
pub use fluent::FluentTranslator;

pub use crate::tr;

/// A translation backend that resolves message keys to localized strings.
///
/// Implement this trait to plug iced-kit into any i18n ecosystem: Fluent,
/// gettext, a custom JSON store, or even a compile-time lookup table. The
/// default methods on [`I18n`] call into this trait, so a single
/// implementation covers all translation needs.
///
/// # Examples
///
/// A minimal identity translator that returns keys unchanged:
///
/// ```
/// use iced_kit::i18n::Translator;
/// use fluent_bundle::FluentValue;
///
/// struct Identity;
///
/// impl Translator for Identity {
///     fn translate(&self, key: &str) -> String {
///         key.to_owned()
///     }
///
///     fn translate_fmt(&self, key: &str, _args: &[(&str, FluentValue)]) -> String {
///         key.to_owned()
///     }
///
///     fn language(&self) -> &str {
///         "en"
///     }
/// }
/// ```
pub trait Translator {
    /// Translates `key` into the current language, with no format arguments.
    ///
    /// If the key is not found, the key itself is returned as a fallback so
    /// that missing translations are visible rather than silently empty.
    fn translate(&self, key: &str) -> String;

    /// Translates `key` with named format arguments.
    ///
    /// Arguments are passed as `(name, value)` pairs where the value is a
    /// [`FluentValue`]. The backend is responsible for inserting them into
    /// the message pattern. Missing keys fall back to the key string.
    ///
    /// [`FluentValue`]: fluent_bundle::FluentValue
    fn translate_fmt(&self, key: &str, args: &[(&str, fluent_bundle::FluentValue)]) -> String;

    /// Returns the current language identifier (e.g. `"en"`, `"zh-CN"`).
    fn language(&self) -> &str;
}

/// A translation context that wraps a [`Translator`] backend.
///
/// The application owns an `I18n` instance, sets the desired language on it,
/// and passes `&self.i18n` (or a local `i18n` binding) to widget
/// constructors via [`tr!`](crate::tr!) or direct method calls.
///
/// # Examples
///
/// ```no_run
/// use iced_kit::i18n::{FluentTranslator, I18n};
///
/// let mut i18n = I18n::new(FluentTranslator::from_str("en", "-ok = OK\n").unwrap());
/// i18n.set_language("zh-CN");
///
/// assert_eq!(i18n.language(), "zh-CN");
/// ```
#[must_use = "an I18n context does nothing unless used to resolve translations"]
pub struct I18n<T: Translator> {
    translator: T,
    language: String,
}

impl<T: Translator> I18n<T> {
    /// Creates a new translation context wrapping `translator`.
    ///
    /// The initial language is taken from the translator's
    /// [`Translator::language`].
    pub fn new(translator: T) -> Self {
        let language = translator.language().to_owned();
        Self {
            translator,
            language,
        }
    }

    /// Switches the active language.
    ///
    /// This updates the label returned by [`language`](Self::language). The
    /// backend is responsible for resolving the new label to translations;
    /// if it does not support dynamic switching the caller should construct a
    /// new [`I18n`] with a translator for the desired language.
    pub fn set_language(&mut self, lang: impl Into<String>) {
        self.language = lang.into();
    }

    /// Returns the current language identifier.
    pub fn language(&self) -> &str {
        &self.language
    }

    /// Translates `key` with no format arguments.
    ///
    /// If the backend cannot resolve `key`, the key itself is returned.
    pub fn tr(&self, key: &str) -> String {
        self.translator.translate(key)
    }

    /// Translates `key` with named format arguments.
    ///
    /// Arguments are passed as `(name, value)` pairs. If the backend cannot
    /// resolve `key`, the key itself is returned.
    pub fn tr_fmt(&self, key: &str, args: &[(&str, fluent_bundle::FluentValue)]) -> String {
        self.translator.translate_fmt(key, args)
    }
}

#[cfg(test)]
mod tests {
    use super::{I18n, Translator};
    use fluent_bundle::FluentValue;

    struct Identity;

    impl Translator for Identity {
        fn translate(&self, key: &str) -> String {
            key.to_owned()
        }

        fn translate_fmt(&self, key: &str, _args: &[(&str, FluentValue)]) -> String {
            key.to_owned()
        }

        fn language(&self) -> &str {
            "en"
        }
    }

    #[test]
    fn new_takes_language_from_translator() {
        let i18n = I18n::new(Identity);
        assert_eq!(i18n.language(), "en");
    }

    #[test]
    fn set_language_overrides() {
        let mut i18n = I18n::new(Identity);
        i18n.set_language("zh-CN");
        assert_eq!(i18n.language(), "zh-CN");
    }

    #[test]
    fn tr_delegates_to_translator() {
        let i18n = I18n::new(Identity);
        assert_eq!(i18n.tr("save"), "save");
    }

    #[test]
    fn tr_fmt_delegates_to_translator() {
        let i18n = I18n::new(Identity);
        let args: Vec<(&str, FluentValue)> = vec![("name", FluentValue::from("Alice"))];
        assert_eq!(i18n.tr_fmt("hello", &args), "hello");
    }
}
