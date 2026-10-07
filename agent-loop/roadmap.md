# Roadmap

- Publish `agent-loop` to crates.io after `llm-wire`: drop `publish = false`, give the `llm-wire` dependency a version, add the crate to `bump.yml` and the `cargo publish --dry-run` step in `ci.yml`, and list `agent-loop/src` in `sonar-project.properties`. Unblocks T2.
