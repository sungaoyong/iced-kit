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

//! Regression: a single close/select must not apply layout mutations twice.

use iced_kit::dock::unstable::{build_tree, dispatch_action};
use iced_kit::dock::{panel, tabs, DockAction, DockSession, DockWidgetState, TabAction};

#[test]
fn dispatch_close_once_removes_panel() {
    let built = build_tree(&tabs([panel("a", "A", 0u32), panel("b", "B", 1u32)])).expect("built");
    let panel_b = built.index.panel_node("b").expect("b");
    let mut state = DockWidgetState::<u32>::from_built(built, None);

    assert!(dispatch_action(
        &mut state,
        DockAction::Tab(TabAction::Close { panel: panel_b })
    ));
    assert!(!state.index.panels.contains_key("b"));
    assert!(state.index.panels.contains_key("a"));

    let second = dispatch_action(
        &mut state,
        DockAction::Tab(TabAction::Close { panel: panel_b }),
    );
    assert!(!second);
}

#[test]
fn session_select_does_not_require_update_handler() {
    let session: DockSession<u32> =
        DockSession::from_tree(tabs([panel("a", "A", 0u32), panel("b", "B", 1u32)]))
            .expect("session");
    session.select_panel("b").expect("select");
    assert_eq!(session.active_panel().as_deref(), Some("b"));
    let count_after_one = session.state().borrow().layout.nodes.len();
    session.select_panel("a").expect("select again");
    let count_after_two = session.state().borrow().layout.nodes.len();
    assert_eq!(count_after_one, count_after_two);
}
