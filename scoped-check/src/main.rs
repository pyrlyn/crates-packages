//! `scoped-check`: run only the check commands a change touches.

mod config;
mod plan;

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use anyhow::Result;
use clap::{Args, Parser, Subcommand};

use config::{Config, DEFAULT_BASE};
use plan::Plan;

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Args)]
struct Common {
    /// Ref to compare against [default: `base` in the config, else origin/main]
    #[arg(long)]
    base: Option<String>,
    /// Config file [default: scoped-check.toml at the git top level]
    #[arg(long)]
    config: Option<PathBuf>,
    /// Select every gate with full-workspace values, ignoring the change set
    #[arg(long)]
    all: bool,
}

#[derive(Subcommand)]
enum Cmd {
    /// Print the gates that would run and their expanded commands
    Plan {
        #[command(flatten)]
        common: Common,
        /// Print one JSON object instead of text
        #[arg(long)]
        json: bool,
    },
    /// Run the selected gates in config order
    Run {
        #[command(flatten)]
        common: Common,
        /// Keep running after a gate fails
        #[arg(long)]
        keep_going: bool,
    },
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("scoped-check: {e:#}");
            ExitCode::from(2)
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode> {
    let (common, json, keep_going, execute) = match cli.command {
        Cmd::Plan { common, json } => (common, json, false, false),
        Cmd::Run { common, keep_going } => (common, false, keep_going, true),
    };
    let cfg = Config::load(&common.config.unwrap_or_else(default_config))?;
    let repo = repo_top();
    let base = common
        .base
        .or_else(|| cfg.base.clone())
        .unwrap_or_else(|| DEFAULT_BASE.to_owned());
    let plan = plan::build(&cfg, &repo, &base, common.all);
    if json {
        println!("{}", serde_json::to_string_pretty(&plan)?);
        return Ok(ExitCode::SUCCESS);
    }
    if plan.nothing_changed {
        println!("nothing changed against {base}");
        return Ok(ExitCode::SUCCESS);
    }
    Ok(if execute {
        execute_plan(&repo, &plan, keep_going)
    } else {
        print_plan(&plan);
        ExitCode::SUCCESS
    })
}

fn default_config() -> PathBuf {
    // Outside git the current directory is the best guess; the change set then fails and
    // every gate runs.
    git_changed_paths::toplevel(Path::new("."))
        .unwrap_or_else(|_| PathBuf::new())
        .join("scoped-check.toml")
}

fn repo_top() -> PathBuf {
    git_changed_paths::toplevel(Path::new(".")).unwrap_or_else(|_| PathBuf::from("."))
}

fn print_plan(plan: &Plan) {
    println!("base: {}", plan.base);
    println!(
        "merge base: {}",
        plan.merge_base.as_deref().unwrap_or("unknown")
    );
    match plan.changed {
        Some(n) => println!("changed paths: {n}"),
        None => println!("changed paths: unknown"),
    }
    for path in &plan.unmatched {
        println!("unmatched: {path}");
    }
    for step in &plan.gates {
        println!("gate {} [{}]: {}", step.name, step.why, step.command);
    }
    for skip in &plan.skipped {
        println!("skip {}: {}", skip.name, skip.reason);
    }
}

fn execute_plan(repo: &Path, plan: &Plan, keep_going: bool) -> ExitCode {
    for skip in &plan.skipped {
        println!("== {}: skipped, {}", skip.name, skip.reason);
    }
    let mut first_failure = None;
    for step in &plan.gates {
        println!("== {}: {}", step.name, step.command);
        let code = shell(&step.command, repo).unwrap_or_else(|e| {
            eprintln!("scoped-check: cannot start `{}`: {e}", step.name);
            1
        });
        if code != 0 {
            first_failure.get_or_insert(code);
            if !keep_going {
                break;
            }
        }
    }
    ExitCode::from(first_failure.unwrap_or(0))
}

/// Exit code of `command` under the platform shell; 1 when it has none (killed by a signal).
fn shell(command: &str, repo: &Path) -> std::io::Result<u8> {
    let (program, flag) = if cfg!(windows) {
        ("cmd", "/C")
    } else {
        ("sh", "-c")
    };
    let status = Command::new(program)
        .arg(flag)
        .arg(command)
        .current_dir(repo)
        .status()?;
    Ok(status.code().map_or(1, |c| u8::try_from(c).unwrap_or(1)))
}
