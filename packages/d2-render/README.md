# d2-render

Render [D2](https://d2lang.com) diagrams from Rust.

By default the crate runs the official `d2` CLI, so every D2 feature, layout
engine (dagre, ELK, TALA) and output format behaves exactly as it does on the
command line. On top of that it gives you:

- a typed API: `Renderer`, `Format`, `RenderOptions`, `RenderReport`;
- structured errors: d2's stderr is parsed into `Diagnostic`s with file,
  line and column;
- SVG metadata: size, `viewBox`, d2 version, and the key of every shape and
  connection in the rendered SVG;
- freshness checks: stamp outputs with a source hash, then find and
  re-render the outdated ones;
- optional features: a pure-Rust `native` backend for a subset of D2, a
  ratatui widget (`tui`), HTML embedding helpers (`web`) and debounced
  re-render on change (`watch`).

## Installing d2

The default backend needs the `d2` binary (v0.9 is what the tests run against):

```bash
brew install d2                                   # macOS / Linux (Homebrew)
curl -fsSL https://d2lang.com/install.sh | sh -s --  # other platforms
```

The binary comes from, in order: `Renderer::with_binary(path)`, then the
`D2_BIN` environment variable, then `d2` on `PATH`. A missing binary is
reported as `Error::BinaryNotFound` by whichever call needs it.

## Usage

```toml
[dependencies]
d2-render = "0.1"
```

```rust
use std::path::Path;
use d2_render::{Error, Format, Layout, RenderOptions, Renderer};

fn main() -> Result<(), Error> {
    let renderer = Renderer::new().options(
        RenderOptions::new()
            .theme(200)          // --theme
            .layout(Layout::Elk) // --layout
            .pad(20)             // --pad
            .sketch(false),      // --sketch
    );

    // From a file. The format comes from the enum, not from the extension.
    let svg = renderer.render_file(Path::new("arch.d2"), Path::new("out/arch.svg"), Format::Svg)?;

    // From a string (written to a temporary .d2 file first).
    renderer.render_str("a -> b: hello", Path::new("out/ab.png"), Format::Png)?;

    // With a full report.
    let report = renderer.render_file_report(Path::new("arch.d2"), &svg, Format::Svg)?;
    println!("{report}"); // rendered arch.d2 -> out/arch.svg (svg, cli backend) in 41.2ms; 315x728, ...
    for key in report.svg.as_ref().unwrap().shapes() {
        println!("shape {key}");
    }

    // Compile errors carry positions.
    match renderer.render_str("a: {", Path::new("out/bad.svg"), Format::Svg) {
        Err(Error::Syntax { diagnostics, .. }) => {
            for d in diagnostics {
                eprintln!("{}:{}: {}", d.line.unwrap_or(0), d.column.unwrap_or(0), d.message);
            }
        }
        other => println!("{other:?}"),
    }
    Ok(())
}
```

`Format` covers everything d2 writes: `Svg`, `Png`, `Pdf`, `Pptx`, `Gif`,
`Txt`. When the output path has another extension, the render goes to a
temporary file beside it and is then moved into place, so the file always
holds the requested format. `Renderer::render_auto` infers the format from
the output extension instead. Missing parent directories are created.

### Options

Every field of `RenderOptions` maps to a flag that `d2 --help` lists (v0.9).
Nothing is passed unless you set it, so d2's own defaults and environment
variables (`D2_THEME`, `D2_LAYOUT`, ...) still apply.

| Builder | d2 flag |
| --- | --- |
| `theme(i64)` / `dark_theme(i64)` | `--theme` / `--dark-theme` |
| `layout(Layout::Dagre \| Elk \| Tala \| Other(..))` | `--layout` |
| `pad(u32)` | `--pad` |
| `sketch(bool)` | `--sketch` |
| `center(bool)` | `--center` |
| `scale(f64)` | `--scale` |
| `timeout_secs(u32)` | `--timeout` |
| `target(str)` | `--target` |
| `no_xml_tag(bool)` | `--no-xml-tag` |
| `omit_version(bool)` | `--omit-version` |
| `salt(str)` | `--salt` |
| `animate_interval_ms(u32)` | `--animate-interval` |
| `stamp(bool)` | none: records a source hash, see [Freshness](#freshness) |

The CLI backend always adds `--watch=false`, so a `D2_WATCH` in the
environment cannot turn a render into a server that never exits.

### Errors

```rust
pub enum Error {
    BinaryNotFound { binary: PathBuf, reason: String }, // d2 not installed / not executable
    Syntax { diagnostics: Vec<Diagnostic>, stderr: String }, // compile errors, with positions
    Io(std::io::Error),
    Failed { code: Option<i32>, messages: Vec<String>, stderr: String, partial: bool }, // any other non-zero exit
    UnsupportedFormat { format: Format, backend: &'static str },
    UnknownFormat(PathBuf),
    Watch(String),
}
```

`Failed` covers bad flags, an unknown theme or layout, timeouts, and
failures such as a remote icon that cannot be fetched; `partial` is set when
d2 still wrote a partial render.

### Structured output

The d2 CLI has no machine-readable output mode (no JSON flag in v0.9), so
the crate parses its stderr. `parse_stderr` is public if you run d2
yourself:

- `err: failed to compile x.d2: /p/x.d2:5:4: maps must be terminated with }`
  becomes `Diagnostic { severity: Error, path: Some("/p/x.d2"), line: Some(5), column: Some(4), message: "maps must be terminated with }" }`;
  every located error is kept (d2 reports several semantic errors at once);
- `warn:` lines become warnings; `success: ... in 4.39ms` gives
  `reported_duration`; `partial render written` sets `partial`.

A successful render returns a `RenderReport`: input, output, format,
duration, backend, warnings, diagnostics, `source_hash`, raw stderr, and for
SVG an `SvgInfo`. `SvgInfo::parse` reads the root `viewBox` and size,
`data-d2-version`, every `id`, and the key of every shape and connection.
d2 writes those keys as base64 in `<g class="...">` (`YQ==` is `a`,
`KGEgLSZndDsgYilbMF0=` is `(a -> b)[0]`); they come back decoded and split
into `shapes()` and `connections()`. `RenderReport` and `SvgInfo` both
implement `Display`.

### Validation and formatting

- `Renderer::validate_file` / `validate_str` run `d2 validate`, then a full
  compile to stdout (`--stdout-format=svg ... -`, with `--bundle=false` so no
  network is touched). Both steps are needed: in v0.9 `d2 validate` only
  parses, so an unknown shape or `opacity: 7` passes it but fails a render.
- `CliBackend::validate_syntax` is the `d2 validate` step on its own.
- `CliBackend::format_file` runs `d2 fmt`; `CliBackend::is_formatted` runs
  `d2 fmt --check`.

### Freshness

With `RenderOptions::stamp(true)` each render records
`sha256(source + options fingerprint)`. SVG gets it in a trailing
`<!-- d2-render:source-sha256=... -->` comment; other formats get a
`<output>.d2hash` sidecar file. Then:

```rust
let r = Renderer::new().options(RenderOptions::new().stamp(true));
r.is_stale(Path::new("a.d2"), Path::new("a.svg"))?;  // bool
r.freshness(Path::new("a.d2"), Path::new("a.svg"))?; // Fresh | Missing | Unstamped | Outdated { .. }
let stale = r.verify_dir(Path::new("docs"), Format::Svg)?; // every docs/**/x.d2 whose x.svg is stale
let reports = r.refresh_dir(Path::new("docs"), Format::Svg)?; // re-render only those
```

Changing a render option (theme, layout, ...) also makes an output stale.
Only the main file is hashed, so changes in files it imports are not
detected.

## Cargo features

| Feature | Adds | Extra dependencies |
| --- | --- | --- |
| *(default)* | CLI backend, diagnostics, reports, SVG metadata, freshness | `sha2`, `tempfile` |
| `native` | `NativeBackend`, `native::render_svg`: pure-Rust subset of D2 to SVG | none |
| `tui` | `tui::RenderStatus`, `tui::RenderStatusWidget` (ratatui `Widget`) | `ratatui` (no default features) |
| `web` | `web::inline_svg`, `data_uri`, `img_tag`, `html_page`, `write_html_page` | none |
| `watch` | `watch::watch`, `watch::watch_channel` | `notify`, `notify-debouncer-mini` |
| `full` | all of the above | |

### `native`

```rust
let svg = d2_render::native::render_svg("a -> b: hi", &RenderOptions::default())?.svg;
let renderer = Renderer::native(); // same API as the CLI renderer, SVG only
```

This is an in-process Rust rewrite of part of D2's pipeline (parser,
compiler, a dagre-style layered layout, SVG writer). No d2 binary, Go or JS
is involved. It covers:

- shapes: rectangle, square, circle, oval, diamond, hexagon, parallelogram,
  cylinder, queue, page, document, step, package, stored_data, person,
  cloud, callout and text (other D2 shapes are drawn as rectangles, with a
  warning);
- labels, including `|md ...|` block strings (shown as plain text), quoted
  keys, nested containers, `_` parent references;
- connections `->`, `<-`, `<->`, `--`, chains (`a -> b -> c`), labels,
  repeated connections;
- `direction`, `width`, `height`, and style keywords `fill`, `stroke`,
  `stroke-width`, `stroke-dash`, `opacity`, `border-radius`, `font-size`,
  `font-color`, `bold`, `italic`, `shadow`, `multiple`, `double-border`;
- D2's "Neutral Default" theme colours and D2's element keys, so
  `SvgInfo` sees the same shape and connection keys as for CLI output
  (a test checks this against the real CLI);
- the same diagnostics as d2 (message, line, column) for the errors it
  checks: unterminated maps, missing destinations, missing values, unknown
  shapes, out-of-range style values, and more.

What it does not do: icons, tooltips, links, `near`, classes, vars, grids,
sequence diagrams, sql_table/class shapes, arrowhead shapes, themes other
than 0, sketch mode, and PNG/PDF output. Unsupported keywords are ignored
with a warning in `RenderReport::warnings`. Imports, globs, `(a -> b)[0]`
references and arrays are rejected with a diagnostic. Positions come from
a different layout algorithm than d2's, so diagrams look similar, not
identical. Use it where installing d2 is not possible (sandboxes, WASM
hosts, CI without network) or for fast previews; use the CLI backend when
the output has to match d2.

### `tui`

```rust
let status = RenderStatus::from_result(Some(input), &renderer.render_file_report(&input, &out, Format::Svg));
frame.render_widget(RenderStatusWidget::new(&status).title("diagram"), area);
```

The widget shows the render state (idle, rendering, done, failed), the
input and output paths, SVG size and element counts, warnings, and each
diagnostic as `line:col message`. It does not preview the image in the
terminal: that needs a rasteriser plus a graphics-protocol crate, which
would outweigh the rest of this crate.

### `web`

```rust
let html = web::inline_svg(&svg_text, None);       // <div class="d2-diagram"><svg ...></div>, XML prolog stripped
let uri = web::data_uri(&png_bytes, Format::Png);  // data:image/png;base64,...
let page = web::html_page("Architecture", &svg_text); // standalone HTML document
```

To put several d2 SVGs on one page, give each one a different
`RenderOptions::salt` so their element ids do not clash.

### `watch`

```rust
let config = WatchConfig { format: Format::Svg, out_dir: None, debounce: Duration::from_millis(200), render_on_start: true };
let (handle, events) = watch::watch_channel(Renderer::new(), &[PathBuf::from("docs")], config)?;
for event in events {
    if let WatchEvent::Rendered { input, result, .. } = event { /* ... */ }
}
// or: watch::watch(renderer, &paths, config, |event| { ... })
```

Directories are watched recursively for `*.d2`. A single file is watched
through its parent directory, so editors that save by renaming still
trigger a render. Outputs go next to their sources, or under `out_dir`
with the same relative layout. Dropping the `WatchHandle` stops the
watcher.

## Examples

```bash
cargo run -p d2-render --example basic -- diagram.d2 out.svg
cargo run -p d2-render --features native --example native -- diagram.d2 out.svg
cargo run -p d2-render --features tui --example tui_report
cargo run -p d2-render --features web --example web_page
cargo run -p d2-render --features watch --example watch -- docs/
```

## Why the CLI

The question was whether D2 could run as a library instead of a
subprocess. These options were checked against the D2 source (Go module
`github.com/d2lang/d2`, formerly `oss.terrastruct.com/d2`) with real builds
(Go 1.27 toolchain, wasmtime 49, October 2026):

| | `d2` CLI (default) | Go library through cgo `c-shared` + FFI | Official JS/WASM build (`d2js`, npm `@d2lang/d2`, formerly `@terrastruct/d2`) | Whole CLI as WASI (`GOOS=wasip1`) under wasmtime | Pure-Rust (`native` feature here, or the `d2-little` crate) |
| --- | --- | --- | --- | --- | --- |
| Feature parity | Complete | Complete | Compile + render to SVG or ASCII; no PNG/PDF | Complete: SVG, PNG, ELK all rendered in the check | Subset (see above) |
| Layouts | dagre, ELK, TALA | dagre, ELK, TALA | dagre, ELK, TALA | dagre, ELK, TALA (dagre and ELK checked) | one dagre-style layered layout |
| Build needs | nothing | Go ≥ 1.27 at build time, hand-written `//export` Go glue (D2 has no C API), a C linker per target, `unsafe` FFI (this repo forbids `unsafe_code`) | `GOOS=js GOARCH=wasm` + `syscall/js`, so it needs Go's `wasm_exec.js` glue and a JS engine | Go ≥ 1.27 to build a 73.5 MB `d2.wasm` from the D2 sources | nothing |
| Runtime needs | `d2` on PATH | Go runtime in-process (the native d2 binary is 56 MB) | Node/Deno/browser; wasmtime/wasmer cannot run `GOOS=js` modules | wasmtime (or the wasmtime crate, a large dependency); first run about 8 s to compile the module, then about 0.5 s per call with the cache | nothing |
| Version drift | whatever d2 is installed; `version()` reports it | pinned at build time | pinned to the npm release | pinned to the `.wasm` you ship | the rewrite has to be kept in sync by hand |
| Ergonomics in Rust | subprocess + stderr parsing (done here) | FFI layer, C ABI, cross-compiling Go | embedding a JS engine | subprocess (same as the CLI) or embedding wasmtime | plain Rust |

Decision: the CLI stays the default backend. It is the only option with
full parity (every layout, PNG/PDF, themes, sketch) that needs no build-time
toolchain. Every library route needs either the Go toolchain plus the D2
sources at build time, or a JS engine, and still pins a D2 version. The
`Backend` trait makes the choice pluggable. The feature-gated `native`
backend covers the cases where no binary can be installed. It is this
crate's own Rust code, with no build or runtime dependency on the D2
repository.

The WASI build does work. If you already have a `d2.wasm` built with
`GOOS=wasip1 GOARCH=wasm go build`, the CLI backend can run it through a
two-line wrapper, because it passes absolute paths and only needs a binary
that behaves like `d2`:

```sh
#!/bin/sh
exec wasmtime run --dir /::/ /path/to/d2.wasm "$@"
```

```rust
let renderer = Renderer::with_binary("/usr/local/bin/d2-wasm"); // or D2_BIN=/usr/local/bin/d2-wasm
```

## Testing

```bash
cargo test -p d2-render --all-features
cargo clippy -p d2-render --all-targets --all-features -- -D warnings
```

Tests that need a real d2 print `SKIP <name>: d2 not available` and pass
when it is missing. PNG tests skip themselves if d2 reports a missing
browser (older d2 releases rendered PNG through Playwright). The error
mapping and the watcher are also tested against scripted stand-ins, so
they run everywhere.

## License

Code written for this crate is MIT, like the rest of this repository's
crates. The files under `src/native/` are a Rust rewrite of parts of D2
(Copyright 2022 Terrastruct, Inc.) and stay under the
[Mozilla Public License 2.0](https://mozilla.org/MPL/2.0/), as stated at
the top of each file; hence `license = "MIT AND MPL-2.0"`.
