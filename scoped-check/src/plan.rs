//! Turns the change set into the commands to run, falling back to "everything" whenever
//! the exact answer is out of reach.

use std::path::PathBuf;

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

/// A selected gate that has nothing to do.
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
    pub gates: Vec<Step>,
    pub skipped: Vec<Skip>,
}

struct Values {
    packages: String,
    filter: String,
    changed: String,
}

/// Plan the gates for `base`. Never returns "nothing to do" because of an error.
pub fn build(cfg: &Config, base: &str, all: bool) -> Plan {
    let mut plan = Plan {
        base: base.to_owned(),
        merge_base: None,
        changed: None,
        nothing_changed: false,
        gates: Vec::new(),
        skipped: Vec::new(),
    };
    let mut forced = all.then(|| "--all".to_owned());
    let mut paths = Vec::new();
    match git_changed_paths::changed_paths(&cfg.dir, base) {
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
        if selection.all {
            let first = selection.unmatched.first().map(|p| p.display().to_string());
            forced = Some(format!("all: unmatched {}", first.unwrap_or_default()));
        } else {
            match scoped(cfg, &paths, &selection) {
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
        changed: String::new(),
    };
    plan.gates = cfg.gates.iter().map(|g| step(g, &full, &why)).collect();
    plan
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

fn scoped(cfg: &Config, paths: &[PathBuf], selection: &Selection) -> Result<Scoped, String> {
    let (mut steps, mut skipped) = (Vec::new(), Vec::new());
    for gate in cfg
        .gates
        .iter()
        .filter(|g| selection.gates.contains(&g.name))
    {
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
            changed: shlex::try_join(words)
                .map_err(|e| format!("cannot quote a changed path: {e}"))?,
        };
        if gate.needs_packages() {
            let affected = affected(cfg, &claimed)?;
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
                    let flags = affected.packages.iter().flat_map(|p| ["-p", p.as_str()]);
                    values.packages = shlex::try_join(flags).map_err(|e| e.to_string())?;
                    let quoted = shlex::try_quote(&filter).map_err(|e| e.to_string())?;
                    values.filter = format!("-E {quoted}");
                }
            }
        }
        steps.push(step(gate, &values, "scoped"));
    }
    Ok((steps, skipped))
}

fn affected(cfg: &Config, claimed: &[&PathBuf]) -> Result<Affected, String> {
    // Changed paths are repo-relative; the cargo library wants them relative to the
    // workspace, so those outside it cannot affect its packages.
    let relative: Vec<PathBuf> = claimed
        .iter()
        .filter_map(|p| p.strip_prefix(&cfg.workspace).ok())
        .map(PathBuf::from)
        .collect();
    cargo_changed_packages::affected(&cfg.dir.join(&cfg.workspace), &relative)
        .map_err(|e| e.to_string())
}
