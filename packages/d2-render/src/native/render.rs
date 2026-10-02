// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SVG output modelled on D2's SVG renderer (d2renderers/d2svg) and shape
// outlines (lib/shape), Copyright 2022 Terrastruct, Inc.

//! SVG writer for a laid-out graph.

use std::fmt::Write as _;

use super::graph::{Graph, Style};
use super::layout::{Layout, Rect};
use super::theme::{Theme, NEUTRAL_DEFAULT};
use crate::svg::{base64_encode, escape_xml};

/// Rendering knobs the native backend honours.
#[derive(Debug, Clone)]
pub struct SvgOptions {
    /// Padding around the diagram.
    pub pad: f64,
    /// Emit `<?xml ...?>`.
    pub xml_tag: bool,
    /// Emit the generator version attribute.
    pub version: bool,
    /// Suffix for ids.
    pub salt: String,
}

impl Default for SvgOptions {
    fn default() -> Self {
        Self {
            pad: 100.0,
            xml_tag: true,
            version: true,
            salt: String::new(),
        }
    }
}

const FONT_FAMILY: &str = "'Source Sans Pro', 'Helvetica Neue', Arial, sans-serif";

fn f(v: f64) -> String {
    let r = (v * 100.0).round() / 100.0;
    if r == r.trunc() {
        format!("{}", r as i64)
    } else {
        format!("{r}")
    }
}

/// Write the SVG document.
pub fn render(g: &Graph, l: &Layout, opts: &SvgOptions) -> String {
    let t: &Theme = &NEUTRAL_DEFAULT;
    let pad = opts.pad;
    let b = l.bounds;
    let (dx, dy) = (pad - b.x, pad - b.y);
    let width = b.w + 2.0 * pad;
    let height = b.h + 2.0 * pad;
    let mut s = String::new();
    if opts.xml_tag {
        s.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>");
    }
    let _ = write!(
        s,
        "<svg xmlns=\"http://www.w3.org/2000/svg\"{} preserveAspectRatio=\"xMinYMin meet\" \
         viewBox=\"0 0 {w} {h}\">",
        if opts.version {
            format!(" data-d2-render-native=\"{}\"", env!("CARGO_PKG_VERSION"))
        } else {
            String::new()
        },
        w = f(width),
        h = f(height),
    );
    let bg = g.objects[0]
        .style
        .fill
        .clone()
        .unwrap_or_else(|| t.n7.to_string());
    let _ = write!(
        s,
        "<rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke-width=\"0\"/>",
        f(width),
        f(height),
        escape_xml(&bg)
    );

    // Arrow markers, one per stroke colour.
    let mut colors: Vec<String> = Vec::new();
    for e in &g.edges {
        let c = e.style.stroke.clone().unwrap_or_else(|| t.b1.to_string());
        if !colors.contains(&c) {
            colors.push(c);
        }
    }
    if !colors.is_empty() {
        s.push_str("<defs>");
        for (i, c) in colors.iter().enumerate() {
            let _ = write!(
                s,
                "<marker id=\"d2n-arrow-{i}{salt}\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" \
                 markerWidth=\"8\" markerHeight=\"8\" markerUnits=\"userSpaceOnUse\" \
                 orient=\"auto-start-reverse\"><path d=\"M0,0 L10,5 L0,10 z\" fill=\"{c}\"/></marker>",
                salt = escape_xml(&opts.salt),
                c = escape_xml(c),
            );
        }
        s.push_str("</defs>");
    }
    let _ = write!(s, "<g transform=\"translate({},{})\">", f(dx), f(dy));

    // Shapes: parents before children.
    let mut order = vec![0usize];
    let mut k = 0;
    while k < order.len() {
        let i = order[k];
        order.extend(g.objects[i].children.iter().copied());
        k += 1;
    }
    for &i in order.iter().skip(1) {
        shape(&mut s, g, l, t, i);
    }
    for (ei, e) in g.edges.iter().enumerate() {
        let r = &l.routes[ei];
        let stroke = e.style.stroke.clone().unwrap_or_else(|| t.b1.to_string());
        let mi = colors.iter().position(|c| *c == stroke).unwrap_or(0);
        let marker = format!("url(#d2n-arrow-{mi}{})", escape_xml(&opts.salt));
        let sw = e.style.stroke_width.unwrap_or(2.0);
        let _ = write!(
            s,
            "<g class=\"{}\"{}>",
            base64_encode(escape_xml(&e.key).as_bytes()),
            opacity_attr(&e.style)
        );
        let _ = write!(
            s,
            "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}{}/>",
            path_d(&r.points),
            escape_xml(&stroke),
            f(sw),
            dash_attr(&e.style, sw),
            if e.src_arrow {
                format!(" marker-start=\"{marker}\"")
            } else {
                String::new()
            },
            if e.dst_arrow {
                format!(" marker-end=\"{marker}\"")
            } else {
                String::new()
            },
        );
        if let (Some(text), Some(lb)) = (&e.label, r.label) {
            let _ = write!(
                s,
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" rx=\"2\"/>",
                f(lb.x),
                f(lb.y),
                f(lb.w),
                f(lb.h),
                t.n7
            );
            let color = e
                .style
                .font_color
                .clone()
                .unwrap_or_else(|| t.n2.to_string());
            text_el(
                &mut s,
                text,
                lb.x + lb.w / 2.0,
                lb.y,
                e.style.font_size.unwrap_or(16.0),
                &color,
                e.style.bold.unwrap_or(false),
                e.style.italic.unwrap_or(true),
            );
        }
        s.push_str("</g>");
    }
    s.push_str("</g></svg>");
    s
}

fn opacity_attr(st: &Style) -> String {
    st.opacity
        .map(|o| format!(" opacity=\"{}\"", f(o)))
        .unwrap_or_default()
}

fn dash_attr(st: &Style, sw: f64) -> String {
    match st.stroke_dash {
        Some(d) if d > 0.0 => {
            let len = d * sw;
            format!(" stroke-dasharray=\"{},{}\"", f(len), f(len))
        }
        _ => String::new(),
    }
}

/// Straight segments for two points, smooth quadratic joins through bends.
fn path_d(pts: &[(f64, f64)]) -> String {
    let mut d = String::new();
    let Some(&(x0, y0)) = pts.first() else {
        return d;
    };
    let _ = write!(d, "M {} {}", f(x0), f(y0));
    if pts.len() == 2 {
        let _ = write!(d, " L {} {}", f(pts[1].0), f(pts[1].1));
        return d;
    }
    for i in 1..pts.len() - 1 {
        let (cx, cy) = pts[i];
        let (nx, ny) = pts[i + 1];
        let (ex, ey) = if i + 1 == pts.len() - 1 {
            (nx, ny)
        } else {
            ((cx + nx) / 2.0, (cy + ny) / 2.0)
        };
        let _ = write!(d, " Q {} {} {} {}", f(cx), f(cy), f(ex), f(ey));
    }
    d
}

#[allow(clippy::too_many_arguments)]
fn text_el(
    s: &mut String,
    text: &str,
    cx: f64,
    top: f64,
    size: f64,
    color: &str,
    bold: bool,
    italic: bool,
) {
    let line_h = size * super::text::LINE_HEIGHT;
    let _ = write!(
        s,
        "<text x=\"{}\" y=\"{}\" fill=\"{}\" font-family=\"{FONT_FAMILY}\" font-size=\"{}\" \
         text-anchor=\"middle\"{}{}>",
        f(cx),
        f(top + line_h * 0.75),
        escape_xml(color),
        f(size),
        if bold { " font-weight=\"bold\"" } else { "" },
        if italic { " font-style=\"italic\"" } else { "" },
    );
    for (i, line) in text.split('\n').enumerate() {
        if i == 0 {
            let _ = write!(s, "<tspan x=\"{}\">{}</tspan>", f(cx), escape_xml(line));
        } else {
            let _ = write!(
                s,
                "<tspan x=\"{}\" dy=\"{}\">{}</tspan>",
                f(cx),
                f(line_h),
                escape_xml(line)
            );
        }
    }
    s.push_str("</text>");
}

/// SVG element(s) for a shape outline.
fn outline(shape: &str, r: Rect, radius: f64, attrs: &str) -> String {
    let Rect { x, y, w, h } = r;
    let poly = |pts: &[(f64, f64)]| {
        let p: Vec<String> = pts
            .iter()
            .map(|(a, b)| format!("{},{}", f(*a), f(*b)))
            .collect();
        format!("<polygon points=\"{}\"{attrs}/>", p.join(" "))
    };
    match shape {
        "circle" | "oval" => format!(
            "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\"{attrs}/>",
            f(x + w / 2.0),
            f(y + h / 2.0),
            f(w / 2.0),
            f(h / 2.0)
        ),
        "diamond" => poly(&[
            (x + w / 2.0, y),
            (x + w, y + h / 2.0),
            (x + w / 2.0, y + h),
            (x, y + h / 2.0),
        ]),
        "hexagon" => {
            let i = (w / 4.0).min(h / 2.0);
            poly(&[
                (x + i, y),
                (x + w - i, y),
                (x + w, y + h / 2.0),
                (x + w - i, y + h),
                (x + i, y + h),
                (x, y + h / 2.0),
            ])
        }
        "parallelogram" => {
            let i = (w * 0.15).min(26.0);
            poly(&[(x + i, y), (x + w, y), (x + w - i, y + h), (x, y + h)])
        }
        "step" => {
            let i = (w * 0.15).min(30.0);
            poly(&[
                (x, y),
                (x + w - i, y),
                (x + w, y + h / 2.0),
                (x + w - i, y + h),
                (x, y + h),
                (x + i, y + h / 2.0),
            ])
        }
        "callout" => {
            let body = h - 20.0;
            poly(&[
                (x, y),
                (x + w, y),
                (x + w, y + body),
                (x + w * 0.45, y + body),
                (x + w * 0.3, y + h),
                (x + w * 0.3, y + body),
                (x, y + body),
            ])
        }
        "page" => {
            let c = (w.min(h) * 0.25).min(20.0);
            format!(
                "<path d=\"M {x0} {y0} L {x1} {y0} L {x2} {y1} L {x2} {y2} L {x0} {y2} Z \
                 M {x1} {y0} L {x1} {y1} L {x2} {y1}\"{attrs}/>",
                x0 = f(x),
                y0 = f(y),
                x1 = f(x + w - c),
                x2 = f(x + w),
                y1 = f(y + c),
                y2 = f(y + h),
            )
        }
        "document" => {
            let wave = 10.0;
            format!(
                "<path d=\"M {} {} L {} {} L {} {} C {} {} {} {} {} {} C {} {} {} {} {} {} Z\"{attrs}/>",
                f(x),
                f(y),
                f(x + w),
                f(y),
                f(x + w),
                f(y + h - wave),
                f(x + w * 0.75),
                f(y + h - 2.0 * wave),
                f(x + w * 0.6),
                f(y + h),
                f(x + w * 0.5),
                f(y + h - wave),
                f(x + w * 0.35),
                f(y + h - 2.0 * wave),
                f(x + w * 0.2),
                f(y + h + wave * 0.5),
                f(x),
                f(y + h - wave),
            )
        }
        "cylinder" => {
            let ry = (h * 0.1).min(12.0);
            format!(
                "<path d=\"M {x0} {ya} A {rx} {ry} 0 0 1 {x1} {ya} L {x1} {yb} A {rx} {ry} 0 0 1 \
                 {x0} {yb} Z M {x0} {ya} A {rx} {ry} 0 0 0 {x1} {ya}\"{attrs}/>",
                x0 = f(x),
                x1 = f(x + w),
                ya = f(y + ry),
                yb = f(y + h - ry),
                rx = f(w / 2.0),
                ry = f(ry),
            )
        }
        "queue" => {
            let rx = (w * 0.1).min(12.0);
            format!(
                "<path d=\"M {xa} {y0} L {xb} {y0} A {rx} {ry} 0 0 1 {xb} {y1} L {xa} {y1} \
                 A {rx} {ry} 0 0 1 {xa} {y0} Z M {xb} {y0} A {rx} {ry} 0 0 0 {xb} {y1}\"{attrs}/>",
                xa = f(x + rx),
                xb = f(x + w - rx),
                y0 = f(y),
                y1 = f(y + h),
                rx = f(rx),
                ry = f(h / 2.0),
            )
        }
        "stored_data" => {
            let c = (w * 0.1).min(15.0);
            format!(
                "<path d=\"M {xa} {y0} L {x1} {y0} Q {xb} {ym} {x1} {y1} L {xa} {y1} Q {x0} {ym} \
                 {xa} {y0} Z\"{attrs}/>",
                xa = f(x + c),
                x0 = f(x - c),
                x1 = f(x + w),
                xb = f(x + w - 2.0 * c),
                y0 = f(y),
                y1 = f(y + h),
                ym = f(y + h / 2.0),
            )
        }
        "package" => {
            let tab_w = (w * 0.4).min(60.0);
            let tab_h = 10.0;
            format!(
                "<path d=\"M {x0} {y0} L {xt} {y0} L {xt} {yt} L {x1} {yt} L {x1} {y1} L {x0} {y1} Z\"{attrs}/>",
                x0 = f(x),
                xt = f(x + tab_w),
                x1 = f(x + w),
                y0 = f(y),
                yt = f(y + tab_h),
                y1 = f(y + h),
            )
        }
        "cloud" => format!(
            "<path d=\"M {a} {b} C {c} {d} {e} {g} {h_} {i} C {j} {k} {l} {m} {n} {o} \
             C {p} {q} {r_} {s_} {t} {u} C {v} {w_} {x_} {y_} {a} {b} Z\"{attrs}/>",
            a = f(x + w * 0.2),
            b = f(y + h * 0.8),
            c = f(x - w * 0.05),
            d = f(y + h * 0.8),
            e = f(x),
            g = f(y + h * 0.35),
            h_ = f(x + w * 0.25),
            i = f(y + h * 0.3),
            j = f(x + w * 0.3),
            k = f(y - h * 0.05),
            l = f(x + w * 0.7),
            m = f(y - h * 0.05),
            n = f(x + w * 0.75),
            o = f(y + h * 0.3),
            p = f(x + w * 1.02),
            q = f(y + h * 0.3),
            r_ = f(x + w * 1.05),
            s_ = f(y + h * 0.8),
            t = f(x + w * 0.8),
            u = f(y + h * 0.8),
            v = f(x + w * 0.7),
            w_ = f(y + h * 1.05),
            x_ = f(x + w * 0.3),
            y_ = f(y + h * 1.05),
        ),
        "person" => {
            let head = (w.min(h - 30.0) * 0.22).max(8.0);
            let cx = x + w / 2.0;
            let body_top = y + 2.0 * head + 6.0;
            let body_bottom = y + h - 30.0;
            format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\"{attrs}/><path d=\"M {} {} Q {} {} {} {} \
                 L {} {} Q {} {} {} {} Z\"{attrs}/>",
                f(cx),
                f(y + head),
                f(head),
                f(x + w * 0.1),
                f(body_bottom),
                f(x + w * 0.1),
                f(body_top),
                f(cx),
                f(body_top),
                f(cx),
                f(body_top),
                f(x + w * 0.9),
                f(body_top),
                f(x + w * 0.9),
                f(body_bottom),
            )
        }
        _ => format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{}{attrs}/>",
            f(x),
            f(y),
            f(w),
            f(h),
            if radius > 0.0 {
                format!(" rx=\"{}\" ry=\"{}\"", f(radius), f(radius))
            } else {
                String::new()
            }
        ),
    }
}

fn shape(s: &mut String, g: &Graph, l: &Layout, t: &Theme, i: usize) {
    let o = &g.objects[i];
    let r = l.boxes[i];
    let st = &o.style;
    let fs = l.font_sizes[i];
    let _ = write!(
        s,
        "<g class=\"{}\"{}>",
        base64_encode(escape_xml(&o.abs).as_bytes()),
        opacity_attr(st)
    );
    if o.shape != "text" {
        let dashed = st.stroke_dash.is_some_and(|d| d > 0.0);
        let fill = st.fill.clone().unwrap_or_else(|| t.fill(g, i).to_string());
        let stroke = st
            .stroke
            .clone()
            .unwrap_or_else(|| t.stroke(g, i, dashed).to_string());
        let sw = st.stroke_width.unwrap_or(2.0);
        let attrs = format!(
            " fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}",
            escape_xml(&fill),
            escape_xml(&stroke),
            f(sw),
            dash_attr(st, sw)
        );
        let radius = st.border_radius.unwrap_or(0.0);
        s.push_str("<g class=\"shape\">");
        if st.shadow == Some(true) {
            let sh = Rect {
                x: r.x + 4.0,
                y: r.y + 4.0,
                ..r
            };
            s.push_str(&outline(
                &o.shape,
                sh,
                radius,
                " fill=\"#000000\" fill-opacity=\"0.12\" stroke=\"none\"",
            ));
        }
        if st.multiple == Some(true) {
            let back = Rect {
                x: r.x + 10.0,
                y: r.y - 10.0,
                ..r
            };
            s.push_str(&outline(&o.shape, back, radius, &attrs));
        }
        s.push_str(&outline(&o.shape, r, radius, &attrs));
        if st.double_border == Some(true) && r.w > 20.0 && r.h > 20.0 {
            let inner = Rect {
                x: r.x + 5.0,
                y: r.y + 5.0,
                w: r.w - 10.0,
                h: r.h - 10.0,
            };
            s.push_str(&outline(&o.shape, inner, radius, &attrs));
        }
        s.push_str("</g>");
    }
    let label = o.label_text();
    if !label.is_empty() {
        let (_, th) = super::text::measure(label, fs, true);
        let color = st.font_color.clone().unwrap_or_else(|| t.n1.to_string());
        let bold = st.bold.unwrap_or(o.shape != "text");
        let italic = st.italic.unwrap_or(false);
        let top = if o.is_container() {
            r.y + 15.0
        } else if o.shape == "person" {
            r.y + r.h - th - 2.0
        } else if o.shape == "cylinder" {
            r.y + (r.h - th) / 2.0 + 6.0
        } else {
            r.y + (r.h - th) / 2.0
        };
        text_el(s, label, r.x + r.w / 2.0, top, fs, &color, bold, italic);
    }
    s.push_str("</g>");
}
