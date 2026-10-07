# Toolchain

| Program | How to install | Why here | Source |
| --- | --- | --- | --- |
| rustc | mise | Build | https://github.com/rust-lang/rust |
| just | mise | Test recipe | https://github.com/casey/just |
| ketch | see its README | Installs swarfr | https://github.com/pyrlyn/ketch |
| swarfr | ketch | Lossless cleanup of target/ after tests | https://github.com/listepo/swarfr |

## cargo

| Package | Where | Source | Why here |
| --- | --- | --- | --- |
| serde | local | https://github.com/serde-rs/serde | Frontmatter deserialization |
| serde_json | local | https://github.com/serde-rs/json | Skill tool schema and output |
| serde_yaml | local | https://github.com/dtolnay/serde-yaml | SKILL.md frontmatter (deprecated upstream, kept for no behaviour change; see ideas.md) |
| thiserror | local | https://github.com/dtolnay/thiserror | Typed errors |
| tempfile | local | https://github.com/Stebalien/tempfile | Skill directory fixtures in tests |

## ketch

| Package | Where | Source | Why here |
| --- | --- | --- | --- |
| swarfr | global | https://github.com/listepo/swarfr | Lossless cleanup of target/ after tests |
