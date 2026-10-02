//! The pure-Rust backend, plus parity checks against the CLI when d2 is
//! installed.

mod common;

use std::collections::BTreeSet;
use std::fs;

use common::{fixture, is_svg, real_d2};
use d2_render::native::{check, render_svg};
use d2_render::{Error, Format, NativeBackend, RenderOptions, Renderer, SvgInfo};

#[test]
fn renders_fixture_without_d2() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("valid.svg");
    let report = Renderer::native()
        .render_file_report(&fixture("valid.d2"), &out, Format::Svg)
        .unwrap();
    assert_eq!(report.backend, "native");
    assert!(is_svg(&fs::read(&out).unwrap()));
    let svg = report.svg.unwrap();
    assert!(svg.width.unwrap() > 200.0);
    assert!(svg.shapes().any(|s| s == "backend.queue"));
    assert!(svg
        .connections()
        .any(|c| c == "backend.(worker -> queue)[0]"));
}

#[test]
fn showcase_renders_every_shape_kind() {
    let src = fs::read_to_string(fixture("native_showcase.d2")).unwrap();
    let out = render_svg(&src, &RenderOptions::default()).unwrap();
    let info = SvgInfo::parse(&out.svg);
    assert_eq!(info.shapes().count(), 10);
    assert_eq!(info.connections().count(), 8);
    assert!(out.svg.contains("<ellipse"));
    assert!(out.svg.contains("<polygon"));
    assert!(out.svg.contains("stroke-dasharray"));
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
}

#[test]
fn png_is_unsupported() {
    let dir = tempfile::tempdir().unwrap();
    let err = Renderer::native()
        .render_str("a", &dir.path().join("a.png"), Format::Png)
        .unwrap_err();
    assert!(matches!(
        err,
        Error::UnsupportedFormat {
            backend: "native",
            ..
        }
    ));
}

#[test]
fn diagnostics_have_positions_and_path() {
    let r = Renderer::with_backend(NativeBackend::new());
    let err = r.validate_file(&fixture("invalid.d2")).unwrap_err();
    let d = &err.diagnostics()[0];
    assert_eq!((d.line, d.column), (Some(5), Some(4)));
    assert_eq!(d.message, "maps must be terminated with }");
    assert!(d.path.as_ref().unwrap().ends_with("invalid.d2"));
    assert!(check("a -> b").is_ok());
}

#[test]
fn unsupported_features_warn_or_fail() {
    let out = render_svg(
        "a.icon: https://x/y.png\na.tooltip: hi\n",
        &RenderOptions::new().sketch(true),
    )
    .unwrap();
    assert!(out.warnings.iter().any(|w| w.contains("icon")));
    assert!(out.warnings.iter().any(|w| w.contains("--sketch")));
    assert!(check("...@other").unwrap_err().is_syntax());
    assert!(check("*.style.fill: red").unwrap_err().is_syntax());
}

#[test]
fn options_salt_pad_and_xml_tag() {
    let out = render_svg(
        "a -> b",
        &RenderOptions::new()
            .pad(0)
            .salt("s1")
            .no_xml_tag(true)
            .omit_version(true),
    )
    .unwrap();
    assert!(out.svg.starts_with("<svg"));
    assert!(out.svg.contains("d2n-arrow-0s1"));
    assert!(!out.svg.contains("data-d2-render-native"));
}

#[test]
fn freshness_works_with_native_backend() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("a.d2");
    fs::write(&src, "a -> b").unwrap();
    let r = Renderer::native().options(RenderOptions::new().stamp(true));
    let reports = r.refresh_dir(dir.path(), Format::Svg).unwrap();
    assert_eq!(reports.len(), 1);
    assert!(r.verify_dir(dir.path(), Format::Svg).unwrap().is_empty());
    fs::write(&src, "a -> b -> c").unwrap();
    assert_eq!(r.verify_dir(dir.path(), Format::Svg).unwrap().len(), 1);
}

/// Same shape and connection keys as the CLI for the fixtures.
#[test]
fn element_keys_match_cli() {
    let Some(cli) = real_d2("element_keys_match_cli") else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    for name in ["valid.d2", "native_showcase.d2"] {
        let a = cli
            .render_file_report(&fixture(name), &dir.path().join("cli.svg"), Format::Svg)
            .unwrap()
            .svg
            .unwrap();
        let b = Renderer::native()
            .render_file_report(&fixture(name), &dir.path().join("native.svg"), Format::Svg)
            .unwrap()
            .svg
            .unwrap();
        let keys = |i: &SvgInfo| -> BTreeSet<String> {
            i.elements.iter().map(|e| e.key.clone()).collect()
        };
        assert_eq!(keys(&a), keys(&b), "{name}");
    }
}

/// Same diagnostics (position and message) as the CLI for the invalid fixtures.
#[test]
fn diagnostics_match_cli() {
    let Some(cli) = real_d2("diagnostics_match_cli") else {
        return;
    };
    let native = Renderer::native();
    for name in ["invalid.d2", "invalid_semantic.d2"] {
        let strip = |e: Error| -> Vec<(Option<u32>, Option<u32>, String)> {
            e.diagnostics()
                .iter()
                .map(|d| (d.line, d.column, d.message.clone()))
                .collect()
        };
        let a = strip(cli.validate_file(&fixture(name)).unwrap_err());
        let b = strip(native.validate_file(&fixture(name)).unwrap_err());
        assert_eq!(a, b, "{name}");
    }
}
