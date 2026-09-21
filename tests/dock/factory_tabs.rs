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

use iced_kit::dock::model::{Layout, NodeKind};
use iced_kit::dock::unstable::Factory;

#[test]
fn fill_and_close_collapses_empty_pane() {
    let factory = Factory;
    let mut layout = Layout::<u32>::new();

    let a = factory.insert_panel(&mut layout, "a", "A", 0u32);
    let b = factory.insert_panel(&mut layout, "b", "B", 1u32);
    let p1 = factory.create_pane(&mut layout);
    let p2 = factory.create_pane(&mut layout);
    factory.add_panel_to_pane(&mut layout, p1, a).unwrap();
    factory.add_panel_to_pane(&mut layout, p2, b).unwrap();

    factory.dock_fill(&mut layout, a, p2).unwrap();
    factory.close(&mut layout, b).unwrap();

    let NodeKind::Pane(p) = layout.kind(p2).unwrap() else {
        panic!("pane remains");
    };
    assert_eq!(p.tabs.len(), 1);
    assert_eq!(p.active, Some(a));
}
