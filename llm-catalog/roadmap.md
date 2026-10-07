# Roadmap

- Publish `llm-catalog` to crates.io once `llm-wire` is published: drop `publish = false`, swap the `llm-wire` path dependency for its version, add it to `bump.yml` and the `cargo publish --dry-run` step in `ci.yml`, and list `llm-catalog/src` in `sonar-project.properties`. Unblocks T1.6.
