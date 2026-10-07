# agent-ext

Skills for an agent host.

- `skills`: `discover` scans `<dir>/*/SKILL.md` (YAML frontmatter, the Agent Skills
  format), `index` renders one line per skill for the prompt, and `SkillTool` is
  the deferred `skill` tool that returns a body on demand with its
  `allowed-tools`. A malformed skill is skipped with a notice.
- `frontmatter`: the `---` header split and parse the skill files share.

```rust
use agent_ext::skills::{discover, index, skill_dirs};

let dirs = skill_dirs(".myagent", None, None, Some(std::path::Path::new(".")));
let found = discover(&dirs);
let prompt_part = index(&found.skills);
```

Extracted from cox's `cox-ext`. The crate is not published yet (`publish = false`).
