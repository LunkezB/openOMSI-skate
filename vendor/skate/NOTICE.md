# Source provenance

`crates/` and `Cargo.toml` are copied, unchanged, from the `skate/` directory of
[chasmlol/2010-rust-rewrite-mashup](https://github.com/chasmlol/2010-rust-rewrite-mashup)
at `f608f85e407ff1b7689d54a9aafdd16e95711ac4`. Its license is retained as
`LICENSE-Apache-2.0`. These crates originated in SK8-ENGINE/skate-3-rust-engine
at `cb7968930f14dad38457e98720d1a274e469eec2`.

`tools/` contains source files from that exact SK8-ENGINE revision. Its GPL-3.0
license is retained as `LICENSE-GPL-3.0`; nested third-party license files are
preserved in `tools/vendor/` and `tools/owned_game/` where supplied upstream.
Test scripts, editor files and executable binaries were omitted.

`crates/omsi-skate/src/rails.rs` in the host repository is adapted from
`crates/render_anim/src/skate/rails.rs` of the mashup revision above: its Vec3
import uses the host's glam version. Detection still receives inches and Z-up.

Credit: dumbad's reverse-engineering research and tools; Chasm and the
SK8-ENGINE contributors; the MW2 mashup contributors; the Skate 3 Custom
Engine Layer contributors; duckyinnit's UTT; chasmlol/skate-3-trick-toolkit.
See the nested license files for their notices.

No original OMSI 2 or Skate 3 game data is included. New integration code is
part of the AGPL-3.0-only openOMSI fork. Vendored source keeps its notices.
