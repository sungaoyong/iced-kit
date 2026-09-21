# Publishing checklist

Findings from a release audit of `iced-kit`. Recorded so the blockers are not
rediscovered later, and so the reasons for each decision stay attached to them.

## Summary

| Item | Status |
| --- | --- |
| Crate name `iced-kit` on crates.io | **Available** (registry returned 404) |
| `LICENSE-MIT` / `LICENSE-APACHE` | Added |
| Package metadata | Complete; `cargo package` reaches dependency resolution |
| docs.rs build (`cargo doc --no-deps --features dock`) | Clean, no warnings |
| `iced_dock` dependency | **Resolved** — inlined at source level, see below |

## Resolved: `iced_dock` is now inlined

`iced_dock` is a third-party crate (github.com/Fee0/iced_dock, MIT) that is not
published to crates.io, which blocked `cargo publish`. Rather than depending on
it, its source is now part of this crate under `src/dock/` — 24 files,
~8,700 lines, gated behind the same `dock` feature.

The port changes only:

- Import paths, so the code resolves as `crate::dock::x` rather than `crate::x`.
- The `serde` feature is renamed `dock-serde`, and `slotmap/serde` is declared
  (layout node ids are slotmap keys).
- The tab strip's bundled SVG icons moved to `assets/` at the repository root,
  with their `include_bytes!` paths updated.
- Upstream's own lint allowances are applied at the module, so the code stays
  comparable to upstream instead of being reformatted to satisfy this crate's
  stricter pedantic rules.

Provenance is recorded in `NOTICE`, and each ported file carries a header
comment naming its source.

Verified after inlining:

```
$ cargo package --allow-dirty --no-verify
   Packaged 110 files, 1.7MiB (758.2KiB compressed)
```

The packaged archive was extracted and, from that copy alone, built and tested:

```
$ cargo build --features dock      # clean
$ cargo test  --features dock      # 289 unit + 110 integration tests pass
```

Upstream's own test suite was migrated with it (12 files, ~2,100 lines in
`tests/dock/`), so the inlined layout logic is covered by the tests written for
it rather than only by this crate's tests.

## Metadata

`Cargo.toml` carries everything crates.io expects:

```toml
name = "iced-kit"
version = "0.1.0"
edition = "2021"
rust-version = "1.85"
description = "A shadcn/ui-flavored component library and design system for iced, porting the gpui-kit visual language"
license = "MIT OR Apache-2.0"
repository = "https://github.com/iced-kit/iced-kit"
keywords = ["iced", "gui", "widgets", "components", "design-system"]
categories = ["gui"]
```

The five keywords are at the limit crates.io allows. `categories` holds only
`gui`, which is the closest match in the crates.io category list for a widget
library.

## Licence and attribution

The crate is dual-licensed `MIT OR Apache-2.0`, matching the Rust convention and
the `iced` ecosystem. Both files are present and carry this project's own
copyright line.

The design language and token values come from third parties, which the README
acknowledges. Nothing is copied verbatim, but the provenance is worth stating
plainly:

| Source | Licence | What came from it |
| --- | --- | --- |
| [gpui-kit](https://github.com/longbridge/gpui-kit) | Apache-2.0 | Token names, values and component naming |
| [shadcn/ui](https://ui.shadcn.com/) | MIT | The semantic token vocabulary |
| [iced-astraui](https://github.com/AstraBrew-Labs/iced-astraui) | MIT | Implementation patterns (tokens, global layers) |
| [shadcn-rs](https://github.com/FerrisMind/shadcn-rs) | MIT / Apache-2.0 | Implementation patterns (data table, charts) |
| [iced_dock](https://github.com/Fee0/iced_dock) | MIT | The docking widget, used as a dependency |

None of these are vendored, so no third-party licence text needs to ship in this
crate's package.

## docs.rs

`cargo doc --no-deps --features dock` completes with no warnings, which is the
build docs.rs performs. One ambiguity was fixed along the way: a link to
`iced::widget::stack`, which is both a function and a macro, needed parentheses
to resolve.

To have docs.rs build the dock module, `Cargo.toml` would need:

```toml
[package.metadata.docs.rs]
all-features = true
```

That is deliberately left out for now: with an unpublished dependency, a docs.rs
build with `--all-features` would fail for everyone. It belongs in the same
change as whichever fix above unblocks publishing.

## What was checked and how

- **Name availability** — `GET https://crates.io/api/v1/crates/iced-kit` returns
  404. Checked again immediately before publishing, since names can be taken at
  any time.
- **Packaging** — `cargo package --allow-dirty --no-verify`; succeeds
  (110 files). The archive was extracted and, from that copy alone, built and
  tested with `--features dock`, so what a consumer downloads is known to work.
- **Package contents** — extracted the `.crate` archive and confirmed both
  licence files, the README and the guides are inside, and that the extracted
  source builds.
- **docs.rs** — `cargo doc --no-deps --features dock` completes with no
  warnings; that is the build docs.rs performs.
- **Licensing and attribution** — see the table above. No third-party code is
  vendored, so no external licence text needs to ship.

## Before publishing

- [x] Resolve the `iced_dock` dependency — inlined
- [ ] Add `[package.metadata.docs.rs] all-features = true`
- [ ] Replace the placeholder `repository` URL with the real one
- [ ] Confirm the name is still free immediately before publishing
- [ ] Run both feature configurations one final time:
      `cargo test` and `cargo test --features dock`
