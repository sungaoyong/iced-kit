// Ported from iced_dock (MIT, https://github.com/Fee0/iced_dock). See NOTICE.
//
// Keeps upstream's lint allowances, declared in its own `Cargo.toml`, rather
// than rewriting test code this project did not author.
#![allow(
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::default_trait_access,
    clippy::similar_names,
    clippy::too_many_arguments,
    clippy::needless_pass_by_value,
    clippy::type_complexity,
    clippy::unused_self
)]

use iced_kit::dock::widget::DockWidgetState;

#[test]
fn default_layout_is_empty() {
    let state = DockWidgetState::<u32>::default();
    assert!(
        state.layout.root_child().is_none(),
        "default dock state should start with an empty layout"
    );
}
