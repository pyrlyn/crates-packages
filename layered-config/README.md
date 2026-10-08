# layered-config

Load TOML configuration from ordered layers. A later layer replaces a leaf and deep-merges a table. A missing file is skipped. `source_of` names the layer that last set each leaf.

`config-schema` is not in this workspace, so this crate stands on its own. Figment 0.10.19 compiles its TOML provider out, so merging uses the `toml` crate.
