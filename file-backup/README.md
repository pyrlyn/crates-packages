# file-backup

Copy a file to `<name>.bak-<unix-seconds>` beside it before replacing it.

Used by `ketch config reset` and by rtok agent host setup. A missing file, or a
sibling `.bak-*` that already holds the same bytes, is not copied again. The
copy is created with `create_new`, so a planted symlink at the backup name
cannot redirect the write. Hard links and symlinks beside the file are not
treated as backups.

```rust
use file_backup::backup;
let _ = backup(std::path::Path::new("config.toml"))?;
```
