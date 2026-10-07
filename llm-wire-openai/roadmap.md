# Roadmap

- Publish `llm-wire-openai` to crates.io together with `llm-wire` and `llm-http`: drop `publish = false`, add it to `bump.yml` and the `cargo publish --dry-run` step in `ci.yml`, and list `llm-wire-openai/src` in `sonar-project.properties`. Unblocks T1.6.
