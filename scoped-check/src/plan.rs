//! Turns the change set into the commands to run, falling back to "everything" whenever
//! the exact answer is out of reach.

use std::path::{Path, PathBuf};

use cargo_changed_packages::Affected;
use path_gates::Selection;
use serde::Serialize;

use crate::config::{Config, Gate, Part};

/// A command that will run.
#[derive(Serialize)]
pub struct Step {
    pub name: String,
    pub command: String,
    /// `scoped`, `all: unmatched <path>`, `all: <error>` or `--all`.
    pub why: String,
}

/// A selected gate that has nothing to do, or a gate the change set did not select.
#[derive(Serialize)]
pub struct Skip {
    pub name: String,
    pub reason: String,
}

/// The outcome of planning; also the `plan --json` document.
#[derive(Serialize)]
pub struct Plan {
    pub base: String,
    pub merge_base: Option<String>,
    /// `None` when the change set could not be computed.
    pub changed: Option<usize>,
    pub nothing_changed: bool,
    /// Paths no gate claimed, in input order. Empty unless `unmatched = "ignore"`.
    pub unmatched: Vec<String>,
    pub gates: Vec<Step>,
    pub skipped: Vec<Skip>,
}

struct Values {
    packages: String,
    filter: String,
    changed: String,
}

/// Plan the gates for `base`. Never returns "nothing to do" because of an error.
pub fn build(cfg: &Config, repo: &Path, base: &str, all: bool) -> Plan {
    let mut plan = Plan {
        base: base.to_owned(),
        merge_base: None,
        changed: None,
        nothing_changed: false,
        unmatched: Vec::new(),
        gates: Vec::new(),
        skipped: Vec::new(),
    };
    let mut forced = all.then(|| "--all".to_owned());
    let mut paths = Vec::new();
    match git_changed_paths::changed_paths(repo, base) {
        Ok(found) => {
            plan.merge_base = Some(found.merge_base);
            paths = found.paths.into_iter().collect::<Vec<_>>();
            plan.changed = Some(paths.len());
        }
        Err(e) => forced = forced.or_else(|| fall_back(&e)),
    }
    if forced.is_none() {
        if paths.is_empty() {
            plan.nothing_changed = true;
            return plan;
        }
        let selection = cfg.rules.select(&paths);
        plan.unmatched = unmatched_names(&selection);
        if selection.all {
            let first = selection.unmatched.first().map(|p| p.display().to_string());
            forced = Some(format!("all: unmatched {}", first.unwrap_or_default()));
        } else {
            match scoped(cfg, repo, &paths, &selection) {
                Ok((gates, skipped)) => {
                    plan.gates = gates;
                    plan.skipped = skipped;
                    return plan;
                }
                Err(e) => forced = fall_back(&e),
            }
        }
    }
    let why = forced.unwrap_or_default();
    let full = Values {
        packages: "--workspace".to_owned(),
        filter: String::new(),
        // `.` is the path analog of `--workspace`: tools that take files scan the tree
        // instead of running on an empty argv and passing.
        changed: ".".to_owned(),
    };
    plan.gates = cfg.gates.iter().map(|g| step(g, &full, &why)).collect();
    plan
}

fn unmatched_names(selection: &Selection) -> Vec<String> {
    selection
        .unmatched
        .iter()
        .map(|p| p.display().to_string())
        .collect()
}

fn fall_back(error: &dyn std::fmt::Display) -> Option<String> {
    eprintln!("scoped-check: warning: {error}; running every gate");
    Some(format!("all: {error}"))
}

fn step(gate: &Gate, values: &Values, why: &str) -> Step {
    let command = gate
        .run
        .iter()
        .map(|part| match part {
            Part::Text(t) => t,
            Part::Packages => &values.packages,
            Part::NextestFilter => &values.filter,
            Part::Changed => &values.changed,
        })
        .map(String::as_str)
        .collect();
    Step {
        name: gate.name.clone(),
        command,
        why: why.to_owned(),
    }
}

type Scoped = (Vec<Step>, Vec<Skip>);

fn scoped(
    cfg: &Config,
    repo: &Path,
    paths: &[PathBuf],
    selection: &Selection,
) -> Result<Scoped, String> {
    let (mut steps, mut skipped) = (Vec::new(), Vec::new());
    for gate in &cfg.gates {
        if !selection.gates.contains(&gate.name) {
            skipped.push(Skip {
                name: gate.name.clone(),
                reason: "not selected".to_owned(),
            });
            continue;
        }
        // Selecting each path alone is how a gate learns which paths it claims.
        let claimed: Vec<&PathBuf> = paths
            .iter()
            .filter(|p| cfg.rules.select([p]).gates.contains(&gate.name))
            .collect();
        let words: Option<Vec<&str>> = claimed.iter().map(|p| p.to_str()).collect();
        let words = words.ok_or("a changed path is not valid UTF-8")?;
        let mut values = Values {
            packages: String::new(),
            filter: String::new(),
            changed: join_words(&words)?,
        };
        if gate.needs_packages() {
            let affected = affected(cfg, repo, &claimed)?;
            if !affected.all && affected.packages.is_empty() {
                skipped.push(Skip {
                    name: gate.name.clone(),
                    reason: "nothing it covers changed".to_owned(),
                });
                continue;
            }
            match affected.nextest_filter() {
                None => values.packages = "--workspace".to_owned(),
                Some(filter) => {
                    let flags: Vec<&str> = affected
                        .packages
                        .iter()
                        .flat_map(|p| ["-p", p.as_str()])
                        .collect();
                    values.packages = join_words(&flags)?;
                    values.filter = format!("-E {}", quote_one(&filter)?);
                }
            }
        }
        steps.push(step(gate, &values, "scoped"));
    }
    Ok((steps, skipped))
}

fn affected(cfg: &Config, repo: &Path, claimed: &[&PathBuf]) -> Result<Affected, String> {
    // Changed paths are repo-relative; the cargo library wants them relative to the
    // workspace, so those outside it cannot affect its packages.
    let relative: Vec<PathBuf> = claimed
        .iter()
        .filter_map(|p| p.strip_prefix(&cfg.workspace).ok())
        .map(PathBuf::from)
        .collect();
    cargo_changed_packages::affected(&repo.join(&cfg.workspace), &relative)
        .map_err(|e| e.to_string())
}

fn join_words(words: &[&str]) -> Result<String, String> {
    if cfg!(windows) {
        Ok(words
            .iter()
            .copied()
            .map(quote_cmd)
            .collect::<Vec<_>>()
            .join(" "))
    } else {
        shlex::try_join(words.iter().copied())
            .map_err(|e| format!("cannot quote a changed path: {e}"))
    }
}

fn quote_one(word: &str) -> Result<String, String> {
    if cfg!(windows) {
        Ok(quote_cmd(word))
    } else {
        shlex::try_quote(word)
            .map(|s| s.into_owned())
            .map_err(|e| e.to_string())
    }
}

/// cmd.exe quoting: double quotes, with inner quotes doubled. Metacharacters
/// outside quotes (`&`, `|`, `>`, …) would otherwise start another command.
fn quote_cmd(s: &str) -> String {
    if s.is_empty() {
        return "\"\"".to_owned();
    }
    let special = |c: char| {
        c.is_ascii_whitespace()
            || matches!(
                c,
                '&' | '|' | '<' | '>' | '^' | '%' | '"' | ',' | ';' | '(' | ')' | '!' | '\''
            )
    };
    if !s.chars().any(special) {
        return s.to_owned();
    }
    format!("\"{}\"", s.replace('"', "\"\""))
}

#[cfg(test)]
mod quote_tests {
    use super::quote_cmd;

    #[test]
    fn quote_cmd_wraps_metacharacters() {
        assert_eq!(quote_cmd("a.txt"), "a.txt");
        assert_eq!(quote_cmd("my file.rs"), "\"my file.rs\"");
        assert_eq!(quote_cmd("foo&bar.rs"), "\"foo&bar.rs\"");
        assert_eq!(quote_cmd("x|whoami"), "\"x|whoami\"");
        assert_eq!(quote_cmd("a\"b"), "\"a\"\"b\"");
    }
}
