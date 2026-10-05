# scoped-check — completed tasks

### T1. Run only the checks a change touches

`scoped-check plan` and `scoped-check run` read `scoped-check.toml`, compute the paths a
change touched with `git-changed-paths`, map them to gates with `path-gates`, and expand
`{packages}`, `{nextest_filter}` and `{changed}` in each gate's command from
`cargo-changed-packages`. Any error computing the change set or the affected packages
falls back to running every gate; a config error exits 2.
