/// Resolves a translation key against an [`I18n`](crate::i18n::I18n)
/// context bound to the variable `i18n` in the current scope.
///
/// # Forms
///
/// ```ignore
/// // No arguments — resolves the key verbatim.
/// let label = tr!("save");
///
/// // With named arguments — passes key/value pairs to the backend.
/// let greeting = tr!("hello_user", "name" => "Alice");
/// let items = tr!("items_count", "count" => 42);
/// ```
///
/// Both forms expand to method calls on the `i18n` variable, so the
/// surrounding scope must have a binding named `i18n` that dereferences
/// to an [`I18n`] instance. In an iced `view` function the typical
/// pattern is:
///
/// ```ignore
/// fn view(&self) -> Element<'_, Message> {
///     let i18n = &self.i18n;
///     button(tr!("save"))
/// }
/// ```
///
/// If the backend cannot resolve the key, the key itself is returned as a
/// visible fallback (never an empty string).
#[macro_export]
macro_rules! tr {
    // No-argument form: tr!("key")
    ($key:expr) => {
        $crate::i18n::I18n::tr(&i18n, $key)
    };

    // Named-argument form: tr!("key", "name" => value, ...)
    (
        $key:expr,
        $($arg_name:expr => $arg_value:expr),+ $(,)?
    ) => {
        $crate::i18n::I18n::tr_fmt(
            &i18n,
            $key,
            &[$(
                (
                    $arg_name,
                    ::fluent_bundle::FluentValue::from($arg_value),
                ),
            )+],
        )
    };
}
