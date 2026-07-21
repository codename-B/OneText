This directory contains local copies of crates that are patched for the
`gpui-ce` port.

`gpui-component` and `gpui-component-assets` are based on their crates.io
`0.5.1` releases. They are vendored because those releases depend on upstream
`gpui 0.2.2`; OneText needs them to resolve `gpui` to the published
`gpui-ce` package so all GPUI types come from the same crate instance.

