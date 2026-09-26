//! Fluent (.ftl) backend for iced-kit's i18n system.
//!
//! This module provides [`FluentTranslator`], a ready-made
//! [`Translator`] implementation powered by the `fluent-bundle` crate.
//! It supports the full Fluent syntax: message attributes, ICU
//! `MessageFormat`, plurals, and gendered messages.
//!
//! Enable with the `i18n-fluent` feature:
//!
//! ```toml
//! iced-kit = { version = "0.1", features = ["i18n-fluent"] }
//! ```

use std::fs;
use std::io;

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use thiserror::Error;

use crate::i18n::Translator;

/// Errors that can occur when constructing a [`FluentTranslator`].
#[derive(Debug, Error)]
pub enum FluentTranslatorError {
    /// The `.ftl` content could not be parsed.
    #[error("FTL parse error: {0}")]
    Parse(String),

    /// The `.ftl` content could not be resolved into a valid bundle.
    #[error("FTL resolution error: {0}")]
    Resolution(String),

    /// The locale directory or file could not be read.
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
}

/// A [`Translator`] backend that resolves keys through a Fluent bundle.
///
/// Each `FluentTranslator` holds a single [`FluentBundle`] for one locale.
/// The application constructs one translator per language and switches
/// between them via [`I18n::set_language`](crate::i18n::I18n::set_language)
/// or by creating a new [`I18n`] with the desired translator.
///
/// # Examples
///
/// ```
/// use iced_kit::i18n::{FluentTranslator, I18n};
///
/// let translator = FluentTranslator::from_str("en", r#"
/// save = Save
/// cancel = Cancel
/// hello-user = Hello, { $name }!
/// "#).expect("valid ftl");
///
/// let i18n = I18n::new(translator);
/// assert_eq!(i18n.tr("save"), "Save");
/// assert_eq!(i18n.tr_fmt("hello-user", &[("name", FluentValue::from("Alice"))]), "Hello, Alice!");
/// ```
#[must_use = "a FluentTranslator does nothing unless wrapped in an I18n context"]
pub struct FluentTranslator {
    bundle: FluentBundle<FluentResource>,
    locale: String,
}

impl FluentTranslator {
    /// Creates a translator from an in-memory `.ftl` string.
    ///
    /// # Errors
    ///
    /// Returns a [`FluentTranslatorError::Parse`] if the `.ftl` content
    /// cannot be parsed, or [`FluentTranslatorError::Resolution`] if it
    /// cannot be resolved into a valid bundle.
    ///
    /// # Examples
    ///
    /// ```
    /// use iced_kit::i18n::FluentTranslator;
    ///
    /// let translator = FluentTranslator::from_str("en", "-ok = OK\n").unwrap();
    /// assert_eq!(translator.language(), "en");
    /// ```
    pub fn from_str(lang: &str, ftl: &str) -> Result<Self, FluentTranslatorError> {
        let resource = FluentResource::try_new(ftl.to_owned())
            .map_err(|(_, errors)| FluentTranslatorError::Parse(format!("{errors:?}")))?;
        let mut bundle = FluentBundle::new(vec![lang.parse().expect("valid langid")]);
        bundle.set_use_isolating(false);
        bundle
            .add_resource(resource)
            .map_err(|errors| FluentTranslatorError::Resolution(format!("{errors:?}")))?;
        Ok(Self {
            bundle,
            locale: lang.to_owned(),
        })
    }

    /// Loads translations from a `main.ftl` file inside a directory.
    ///
    /// The directory should contain one subdirectory per locale, each with
    /// a `main.ftl` file. For example:
    ///
    /// ```text
    /// locales/
    ///   en/
    ///     main.ftl
    ///   zh-CN/
    ///     main.ftl
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] if the directory or file cannot be read.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use iced_kit::i18n::FluentTranslator;
    ///
    /// let translator = FluentTranslator::load_from_dir("en", "locales/").unwrap();
    /// assert_eq!(translator.language(), "en");
    /// ```
    pub fn load_from_dir(lang: &str, dir: &str) -> io::Result<Self> {
        let path = format!("{dir}/{lang}/main.ftl");
        let ftl = fs::read_to_string(path)?;
        Self::from_str(lang, &ftl).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}

impl Translator for FluentTranslator {
    fn translate(&self, key: &str) -> String {
        let Some(message) = self.bundle.get_message(key) else {
            return key.to_owned();
        };

        let Some(pattern) = message.value() else {
            return key.to_owned();
        };

        let mut errors = Vec::new();
        let value = self.bundle.format_pattern(pattern, None, &mut errors);

        if errors.is_empty() {
            value.into_owned()
        } else {
            key.to_owned()
        }
    }

    fn translate_fmt(&self, key: &str, args: &[(&str, FluentValue)]) -> String {
        let Some(message) = self.bundle.get_message(key) else {
            return key.to_owned();
        };

        let Some(pattern) = message.value() else {
            return key.to_owned();
        };

        let mut fluent_args = FluentArgs::new();
        for (name, value) in args {
            fluent_args.set(*name, value.clone());
        }

        let mut errors = Vec::new();
        let value = self
            .bundle
            .format_pattern(pattern, Some(&fluent_args), &mut errors);

        if errors.is_empty() {
            value.into_owned()
        } else {
            key.to_owned()
        }
    }

    fn language(&self) -> &str {
        &self.locale
    }
}

#[cfg(test)]
mod tests {
    use super::FluentTranslator;
    use crate::i18n::{FluentTranslator as Reexported, I18n, Translator};
    use fluent_bundle::FluentValue;

    /// Strips Unicode directional isolation marks (U+2068, U+2069) that
    /// fluent-bundle 0.16 inserts around placeables, so assertions compare
    /// the visible string.
    fn clean(s: &str) -> String {
        s.replace('\u{2068}', "").replace('\u{2069}', "")
    }

    #[test]
    fn from_str_resolves_simple_keys() {
        let t = FluentTranslator::from_str("en", "save = Save\n").unwrap();
        assert_eq!(clean(&t.translate("save")), "Save");
    }

    #[test]
    fn from_str_rejects_invalid_ftl() {
        assert!(FluentTranslator::from_str("en", "not valid ftl {{{").is_err());
    }

    #[test]
    fn missing_key_falls_back_to_key() {
        let t = FluentTranslator::from_str("en", "ok = OK\n").unwrap();
        assert_eq!(t.translate("missing"), "missing");
    }

    #[test]
    fn translate_fmt_inserts_arguments() {
        let t = FluentTranslator::from_str("en", "hello = Hello, { $name }!\n").unwrap();
        let args = [("name", FluentValue::from("World"))];
        assert_eq!(clean(&t.translate_fmt("hello", &args)), "Hello, World!");
    }

    #[test]
    fn language_returns_constructor_value() {
        let t = FluentTranslator::from_str("zh-CN", "ok = 确定\n").unwrap();
        assert_eq!(t.language(), "zh-CN");
    }

    #[test]
    fn reexport_matches_inner_type() {
        let a = FluentTranslator::from_str("en", "ok = OK\n").unwrap();
        let _: Reexported = a;
    }

    #[test]
    fn i18n_context_translates() {
        let i18n = I18n::new(FluentTranslator::from_str("en", "save = Save\n").unwrap());
        assert_eq!(clean(&i18n.tr("save")), "Save");
    }
}
