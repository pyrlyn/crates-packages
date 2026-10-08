# abi-drift

Regenerate a cbindgen header and csbindgen C# bindings, then diff them against the committed files. Set `BLESS` (or pass another variable name to `check_env`) to rewrite the committed file. A mismatch error includes the differing lines.
