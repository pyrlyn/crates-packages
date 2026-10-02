//! Render with the pure-Rust backend; no d2 binary needed.
//!
//! cargo run -p d2-render --features native --example native -- [input.d2] [output.svg]

use d2_render::native::render_svg;
use d2_render::{RenderOptions, SvgInfo};

const DEMO: &str = "direction: right
cache: {shape: cylinder}
app: App {
  api -> worker: jobs
}
user: {shape: person}
user -> app.api: HTTPS
app.worker -> cache
";

fn main() -> Result<(), d2_render::Error> {
    let mut args = std::env::args().skip(1);
    let source = match args.next() {
        Some(path) => std::fs::read_to_string(path)?,
        None => DEMO.to_string(),
    };
    let output = args
        .next()
        .unwrap_or_else(|| "target/d2-native-example.svg".into());
    let out = render_svg(&source, &RenderOptions::default())?;
    for w in &out.warnings {
        eprintln!("warning: {w}");
    }
    let info = SvgInfo::parse(&out.svg);
    if let Some(dir) = std::path::Path::new(&output).parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(&output, &out.svg)?;
    println!("wrote {output} ({info})");
    Ok(())
}
