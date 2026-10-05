//! End-to-end tests against a throwaway git repository holding a tiny cargo workspace
//! (`a` depends on `b`, `c` is unrelated). Gates `echo` their expanded commands, so stdout
//! shows what would run. They need a POSIX `sh`.
#![cfg(unix)]

use std::fs;
use std::path::Path;
use std::process::Command as Std;

use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin_cmd;
use tempfile::TempDir;

const CONFIG: &str = r#"
base = "main"

[[gate]]
name = "docs"
paths = ["**/*.md"]
run = "echo docs {changed}"

[[gate]]
name = "test"
paths = ["**/*.rs", "**/Cargo.toml", "Cargo.lock"]
run = "echo test {packages} {nextest_filter}"

[[gate]]
name = "boom"
paths = ["**/*.txt"]
run = "echo boom; exit 3"

[[gate]]
name = "after"
paths = ["**/*.txt"]
run = "echo after"
"#;

fn git(dir: &Path, args: &[&str]) {
    let status = Std::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}

fn write(dir: &Path, path: &str, text: &str) {
    let full = dir.join(path);
    fs::create_dir_all(full.parent().unwrap()).unwrap();
    fs::write(full, text).unwrap();
}

fn member(dir: &Path, name: &str, dep: Option<&str>) {
    let deps = dep.map_or(String::new(), |d| {
        format!("{d} = {{ path = \"../{d}\" }}\n")
    });
    let manifest = format!(
        "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n{deps}"
    );
    write(dir, &format!("{name}/Cargo.toml"), &manifest);
    write(dir, &format!("{name}/src/lib.rs"), "");
}

fn fixture(config: &str) -> TempDir {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    git(dir, &["init", "-q", "-b", "main"]);
    git(dir, &["config", "user.name", "t"]);
    git(dir, &["config", "user.email", "t@example.com"]);
    write(
        dir,
        "Cargo.toml",
        "[workspace]\nmembers = [\"a\", \"b\", \"c\"]\nresolver = \"2\"\n",
    );
    member(dir, "a", Some("b"));
    member(dir, "b", None);
    member(dir, "c", None);
    write(dir, "README.md", "# fixture\n");
    write(dir, "scoped-check.toml", config);
    // The lock file is committed so later `cargo metadata` runs leave the tree clean.
    let status = Std::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .args(["generate-lockfile", "--offline"])
        .current_dir(dir)
        .status()
        .unwrap();
    assert!(status.success());
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", "base"]);
    tmp
}

fn tool(dir: &Path, args: &[&str]) -> Command {
    let mut cmd = cargo_bin_cmd!("scoped-check");
    cmd.args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("CARGO_NET_OFFLINE", "true");
    cmd
}

fn stdout_of(dir: &Path, args: &[&str]) -> String {
    let out = tool(dir, args).assert().success().get_output().clone();
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn docs_only_change_runs_only_the_docs_gate() {
    let repo = fixture(CONFIG);
    write(repo.path(), "README.md", "# changed\n");
    let out = stdout_of(repo.path(), &["run"]);
    assert!(out.contains("== docs: echo docs README.md"), "{out}");
    assert!(out.contains("docs README.md\n"), "{out}");
    assert!(!out.contains("== test"), "{out}");
}

#[test]
fn change_in_b_selects_b_and_its_dependent() {
    let repo = fixture(CONFIG);
    write(repo.path(), "b/src/lib.rs", "// edit\n");
    let out = stdout_of(repo.path(), &["run"]);
    assert!(
        out.contains("test -p a -p b -E 'package(=a) | package(=b)'"),
        "{out}"
    );
    assert!(!out.contains("== docs"), "{out}");
}

#[test]
fn change_in_c_selects_only_c() {
    let repo = fixture(CONFIG);
    write(repo.path(), "c/src/lib.rs", "// edit\n");
    let out = stdout_of(repo.path(), &["plan"]);
    assert!(
        out.contains("gate test [scoped]: echo test -p c -E 'package(=c)'"),
        "{out}"
    );
}

#[test]
fn lock_file_change_selects_the_whole_workspace() {
    let repo = fixture(CONFIG);
    let lock = repo.path().join("Cargo.lock");
    fs::write(
        &lock,
        format!("{}# edit\n", fs::read_to_string(&lock).unwrap()),
    )
    .unwrap();
    let out = stdout_of(repo.path(), &["run"]);
    assert!(out.contains("== test: echo test --workspace"), "{out}");
}

#[test]
fn unknown_base_warns_and_runs_everything() {
    let repo = fixture(CONFIG);
    let assert = tool(repo.path(), &["plan", "--base", "nope"])
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let err = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(err.contains("warning") && err.contains("nope"), "{err}");
    assert!(
        out.contains("gate docs [all: unknown base ref: nope]"),
        "{out}"
    );
    assert!(out.contains("echo test --workspace"), "{out}");
    assert!(out.contains("gate after"), "{out}");
}

#[test]
fn all_flag_ignores_the_change_set() {
    let repo = fixture(CONFIG);
    let out = stdout_of(repo.path(), &["plan", "--all"]);
    assert!(out.contains("gate docs [--all]: echo docs \n"), "{out}");
    assert!(
        out.contains("gate test [--all]: echo test --workspace \n"),
        "{out}"
    );
}

#[test]
fn unmatched_path_selects_every_gate() {
    let repo = fixture(CONFIG);
    write(repo.path(), "blob.bin", "x");
    let out = stdout_of(repo.path(), &["plan"]);
    assert!(out.contains("gate docs [all: unmatched blob.bin]"), "{out}");
}

#[test]
fn nothing_changed_exits_zero() {
    let repo = fixture(CONFIG);
    assert_eq!(
        stdout_of(repo.path(), &["run"]),
        "nothing changed against main\n"
    );
}

#[test]
fn gate_without_affected_packages_is_skipped() {
    // A root README sits outside every package, so a cargo gate claiming it has no work.
    let config = "[[gate]]\nname = \"test\"\npaths = [\"**/*.md\"]\nrun = \"echo {packages}\"\n";
    let repo = fixture(config);
    write(repo.path(), "README.md", "# changed\n");
    let out = stdout_of(repo.path(), &["plan", "--base", "main"]);
    assert!(
        out.contains("skip test: nothing it covers changed"),
        "{out}"
    );
    assert!(!out.contains("gate test"), "{out}");
}

#[test]
fn failing_gate_stops_the_run_with_its_exit_code() {
    let repo = fixture(CONFIG);
    write(repo.path(), "notes.txt", "x");
    let assert = tool(repo.path(), &["run"]).assert().code(3);
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(out.contains("== boom: echo boom; exit 3"), "{out}");
    assert!(!out.contains("== after"), "{out}");
}

#[test]
fn keep_going_runs_the_rest_and_keeps_the_first_failure() {
    let repo = fixture(CONFIG);
    write(repo.path(), "notes.txt", "x");
    let assert = tool(repo.path(), &["run", "--keep-going"]).assert().code(3);
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(out.contains("== after: echo after"), "{out}");
}

#[test]
fn plan_json_has_the_documented_shape() {
    let repo = fixture(CONFIG);
    write(repo.path(), "b/src/lib.rs", "// edit\n");
    let json: serde_json::Value =
        serde_json::from_str(&stdout_of(repo.path(), &["plan", "--json"])).unwrap();
    assert_eq!(json["base"], "main");
    assert_eq!(json["merge_base"].as_str().unwrap().len(), 40);
    assert_eq!(json["changed"], 1);
    assert_eq!(json["nothing_changed"], false);
    assert_eq!(json["gates"][0]["name"], "test");
    assert_eq!(json["gates"][0]["why"], "scoped");
    assert!(
        json["gates"][0]["command"]
            .as_str()
            .unwrap()
            .contains("-p a -p b")
    );
    assert_eq!(json["skipped"], serde_json::json!([]));
}

#[test]
fn config_errors_exit_two() {
    for (config, message) in [
        ("[[gate]]\nname = \"x\"\npaths = [\"*\"]\n", "no `run`"),
        (
            "[[gate]]\nname = \"x\"\nrun = \"echo {nope}\"\n",
            "unknown placeholder",
        ),
        (
            "[[gate]]\nname = \"x\"\nrun = \"true\"\nbogus = [\n",
            "invalid config",
        ),
    ] {
        let repo = fixture(config);
        let assert = tool(repo.path(), &["plan"]).assert().code(2);
        let err = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
        assert!(err.contains(message), "{err}");
    }
}

#[test]
fn shell_braces_stay_literal() {
    let config =
        "[[gate]]\nname = \"x\"\npaths = [\"**/*.md\"]\nrun = \"echo ${HOME:+set} {a,b}\"\n";
    let repo = fixture(config);
    write(repo.path(), "README.md", "# changed\n");
    let out = stdout_of(repo.path(), &["plan", "--base", "main"]);
    assert!(out.contains("echo ${HOME:+set} {a,b}"), "{out}");
}
