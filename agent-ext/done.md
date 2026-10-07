# agent-ext — completed tasks

### T1. Extract skills from cox-ext

Moved out of cox's `crates/cox-ext` (`skills.rs`, `frontmatter.rs`; aulo task T1.17). Skills: `SKILL.md` discovery with later directories overriding earlier ones, the `# Skills` index line, and the deferred `skill` tool as a host-neutral `SkillTool` (`spec`, `subject`, `call`). `skill_dirs` takes the host's project directory name instead of hard-coding `.cox`. No behaviour change otherwise; the cox skills tests moved to `tests/skills.rs` with their fixtures. Check: `cargo test -p agent-ext`, `cargo clippy -p agent-ext --all-targets -- -D warnings` green on Rust 1.99.
