# Roadmap

- Publish `llm-http` to crates.io together with `llm-wire`: drop `publish = false`, add it to `bump.yml` and the `cargo publish --dry-run` step in `ci.yml`, and list `llm-http/src` in `sonar-project.properties`. Unblocks T2.
