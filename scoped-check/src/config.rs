//! `scoped-check.toml`: this tool's own keys, parsed once, next to the gate rules that
//! `path-gates` compiles from the same tables.

use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use path_gates::{GateSpec, Rules, Unmatched};
use serde::Deserialize;

/// Base ref used when neither `--base` nor the config names one.
pub const DEFAULT_BASE: &str = "origin/main";

#[derive(Deserialize)]
struct Raw {
    base: Option<String>,
    workspace: Option<PathBuf>,
    #[serde(default)]
    unmatched: Unmatched,
    #[serde(default)]
    gate: Vec<RawGate>,
}

#[derive(Deserialize)]
struct RawGate {
    name: String,
    run: Option<String>,
    #[serde(default)]
    paths: Vec<String>,
    #[serde(default)]
    always: bool,
}

/// One piece of a gate's `run` template.
pub enum Part {
    Text(String),
    Packages,
    NextestFilter,
    Changed,
}

/// A gate's name and command template.
pub struct Gate {
    pub name: String,
    pub run: Vec<Part>,
}

impl Gate {
    /// Whether the command needs the affected cargo packages (and so a `cargo metadata` run).
    pub fn needs_packages(&self) -> bool {
        self.run
            .iter()
            .any(|p| matches!(p, Part::Packages | Part::NextestFilter))
    }
}

/// The parsed configuration.
pub struct Config {
    pub base: Option<String>,
    /// Cargo workspace root, relative to the git top; empty for the repository itself.
    pub workspace: PathBuf,
    pub rules: Rules,
    /// Gates in config order.
    pub gates: Vec<Gate>,
}

impl Config {
    /// Read and validate the file at `path`.
    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("cannot read config {}", path.display()))?;
        Self::parse(&text).with_context(|| format!("invalid config {}", path.display()))
    }

    fn parse(text: &str) -> Result<Self> {
        let raw: Raw = toml::from_str(text)?;
        let mut specs = Vec::with_capacity(raw.gate.len());
        let mut gates = Vec::with_capacity(raw.gate.len());
        for gate in raw.gate {
            specs.push(GateSpec {
                name: gate.name.clone(),
                paths: gate.paths,
                always: gate.always,
            });
            let Some(run) = gate.run else {
                bail!("gate `{}` has no `run`", gate.name);
            };
            let run = parse_template(&run).with_context(|| format!("gate `{}`", gate.name))?;
            gates.push(Gate {
                name: gate.name,
                run,
            });
        }
        let rules = Rules::new(specs, raw.unmatched)?;
        // `.` components are dropped so the empty path means "the repository top" and
        // `strip_prefix` works on it.
        let mut workspace = PathBuf::new();
        for component in raw.workspace.iter().flat_map(|w| w.components()) {
            match component {
                Component::CurDir => {}
                Component::Normal(part) => workspace.push(part),
                _ => bail!("`workspace` must be a relative path inside the repository"),
            }
        }
        Ok(Self {
            base: raw.base,
            workspace,
            rules,
            gates,
        })
    }
}

// Only `{identifier}` is a placeholder. Other braces (`awk '{print $1}'`, `{a,b}`) and
// `${VAR}` stay literal so ordinary shell commands need no escaping.
fn parse_template(run: &str) -> Result<Vec<Part>> {
    let mut parts = Vec::new();
    let mut text = String::new();
    let mut rest = run;
    while let Some(open) = rest.find('{') {
        let (head, tail) = rest.split_at(open);
        text.push_str(head);
        let name = tail[1..].split_once('}').map(|(name, _)| name);
        match name.filter(|n| is_identifier(n) && !text.ends_with('$')) {
            Some(name) => {
                let part = match name {
                    "packages" => Part::Packages,
                    "nextest_filter" => Part::NextestFilter,
                    "changed" => Part::Changed,
                    other => bail!("unknown placeholder `{{{other}}}`"),
                };
                if !text.is_empty() {
                    parts.push(Part::Text(std::mem::take(&mut text)));
                }
                parts.push(part);
                rest = &tail[name.len() + 2..];
            }
            None => {
                text.push('{');
                rest = &tail[1..];
            }
        }
    }
    text.push_str(rest);
    if !text.is_empty() {
        parts.push(Part::Text(text));
    }
    Ok(parts)
}

fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}
