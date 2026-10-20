# Local crate patches

`gpui-component` is the published 0.8.0 crate with one change in
`src/input/overlay.rs`: the search panel's "query echo" check no longer
compares the session's `anchor_offset`, which flips on every keystroke and
made the panel re-select the query after each typed character.

Drop the `[patch.crates-io]` entry in `Cargo.toml` once a gpui-component
release includes the fix.
