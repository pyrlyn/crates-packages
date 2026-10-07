# Ideas

- Replace `serde_yaml` (deprecated upstream) with a maintained YAML crate such as `serde-saphyr`. The skill `metadata` values are re-serialised from YAML today, so the swap changes their text for non-string values; decide that with the cox owner first.
- aulo loads its skills and shell hooks through this crate.
