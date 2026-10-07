# Roadmap

- Publish `llm-anthropic` to crates.io together with `llm-http` and `llm-wire`: drop `publish = false`, add it to `bump.yml` and the `cargo publish --dry-run` step in `ci.yml`, and list `llm-anthropic/src` in `sonar-project.properties`. Unblocks T2.
