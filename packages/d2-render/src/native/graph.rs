// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Rust rewrite of parts of D2's compiler and graph model (d2compiler,
// d2graph, d2ast keyword tables), Copyright 2022 Terrastruct, Inc.

//! Turns the parsed AST into a graph of objects and connections.

use std::collections::HashMap;

use super::parser::{EdgeOp, KeyPath, Map, Pos, Seg, Stmt, Value};
use crate::diagnostic::Diagnostic;

/// Shapes D2 knows (d2target).
pub const SHAPES: &[&str] = &[
    "rectangle",
    "square",
    "page",
    "parallelogram",
    "document",
    "cylinder",
    "queue",
    "package",
    "step",
    "callout",
    "stored_data",
    "person",
    "c4-person",
    "diamond",
    "oval",
    "circle",
    "hexagon",
    "cloud",
    "text",
    "code",
    "class",
    "sql_table",
    "image",
    "sequence_diagram",
    "hierarchy",
];

/// Shapes the native renderer draws itself; the rest fall back to a
/// rectangle with a warning.
pub const NATIVE_SHAPES: &[&str] = &[
    "rectangle",
    "square",
    "circle",
    "oval",
    "diamond",
    "hexagon",
    "parallelogram",
    "cylinder",
    "queue",
    "page",
    "document",
    "step",
    "package",
    "stored_data",
    "person",
    "cloud",
    "callout",
    "text",
];

const SIMPLE_KEYWORDS: &[&str] = &[
    "label",
    "shape",
    "icon",
    "constraint",
    "tooltip",
    "link",
    "near",
    "width",
    "height",
    "direction",
    "top",
    "left",
    "grid-rows",
    "grid-columns",
    "grid-gap",
    "vertical-gap",
    "horizontal-gap",
    "class",
    "vars",
];

const COMPOSITE_KEYWORDS: &[&str] = &[
    "source-arrowhead",
    "target-arrowhead",
    "classes",
    "constraint",
    "label",
    "icon",
    "tooltip",
];

const STYLE_KEYWORDS: &[&str] = &[
    "opacity",
    "stroke",
    "fill",
    "fill-pattern",
    "stroke-width",
    "stroke-dash",
    "border-radius",
    "font",
    "font-size",
    "font-color",
    "bold",
    "italic",
    "underline",
    "text-transform",
    "shadow",
    "multiple",
    "double-border",
    "3d",
    "animated",
    "filled",
];

fn is_reserved(s: &Seg) -> bool {
    !s.quoted
        && (s.text == "style"
            || SIMPLE_KEYWORDS.contains(&s.text.as_str())
            || COMPOSITE_KEYWORDS.contains(&s.text.as_str()))
}

fn is_style_keyword(s: &Seg) -> bool {
    !s.quoted && STYLE_KEYWORDS.contains(&s.text.as_str())
}

/// Visual style of a shape or connection.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Style {
    /// `style.fill`.
    pub fill: Option<String>,
    /// `style.stroke`.
    pub stroke: Option<String>,
    /// `style.stroke-width`.
    pub stroke_width: Option<f64>,
    /// `style.stroke-dash`.
    pub stroke_dash: Option<f64>,
    /// `style.opacity`.
    pub opacity: Option<f64>,
    /// `style.border-radius`.
    pub border_radius: Option<f64>,
    /// `style.font-size`.
    pub font_size: Option<f64>,
    /// `style.font-color`.
    pub font_color: Option<String>,
    /// `style.bold`.
    pub bold: Option<bool>,
    /// `style.italic`.
    pub italic: Option<bool>,
    /// `style.underline`.
    pub underline: Option<bool>,
    /// `style.shadow`.
    pub shadow: Option<bool>,
    /// `style.double-border`.
    pub double_border: Option<bool>,
    /// `style.multiple`.
    pub multiple: Option<bool>,
    /// `style.animated` (connections).
    pub animated: Option<bool>,
}

/// Layout direction of a board or container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    /// Top to bottom (default).
    #[default]
    Down,
    /// Bottom to top.
    Up,
    /// Left to right.
    Right,
    /// Right to left.
    Left,
}

/// A shape. Index 0 of [`Graph::objects`] is the root board.
#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    /// Last key segment.
    pub id: String,
    /// Full key, e.g. `grp.c`.
    pub abs: String,
    /// Parent index (root is its own parent).
    pub parent: usize,
    /// Children in declaration order.
    pub children: Vec<usize>,
    /// Explicit label (`None` means the id).
    pub label: Option<String>,
    /// Label came from a block string.
    pub label_block: bool,
    /// Shape name (`rectangle` when unset).
    pub shape: String,
    /// Style.
    pub style: Style,
    /// `direction`.
    pub direction: Option<Direction>,
    /// `width`.
    pub width: Option<f64>,
    /// `height`.
    pub height: Option<f64>,
    /// Depth: root is 0.
    pub level: usize,
}

impl Object {
    /// Label text to draw.
    pub fn label_text(&self) -> &str {
        self.label.as_deref().unwrap_or(&self.id)
    }

    /// Has children.
    pub fn is_container(&self) -> bool {
        !self.children.is_empty()
    }
}

/// A connection.
#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
    /// Source object.
    pub src: usize,
    /// Destination object.
    pub dst: usize,
    /// Arrowhead at the source.
    pub src_arrow: bool,
    /// Arrowhead at the destination.
    pub dst_arrow: bool,
    /// Label.
    pub label: Option<String>,
    /// Style.
    pub style: Style,
    /// D2 key, e.g. `grp.(a -> b)[0]`.
    pub key: String,
}

/// A compiled board.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Graph {
    /// Objects; 0 is the root.
    pub objects: Vec<Object>,
    /// Connections in declaration order.
    pub edges: Vec<Edge>,
    /// Non-fatal notes about unsupported features.
    pub warnings: Vec<String>,
}

impl Graph {
    fn new() -> Self {
        Graph {
            objects: vec![Object {
                id: String::new(),
                abs: String::new(),
                parent: 0,
                children: Vec::new(),
                label: None,
                label_block: false,
                shape: "rectangle".into(),
                style: Style::default(),
                direction: None,
                width: None,
                height: None,
                level: 0,
            }],
            edges: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Ancestors of `i` from its parent up to (and including) the root.
    pub fn ancestors(&self, mut i: usize) -> Vec<usize> {
        let mut v = Vec::new();
        while i != 0 {
            i = self.objects[i].parent;
            v.push(i);
        }
        v
    }

    /// `a` is `b` or contains it.
    pub fn contains(&self, a: usize, b: usize) -> bool {
        a == b || a == 0 || self.ancestors(b).contains(&a)
    }
}

/// Compile a parsed file.
pub fn compile(map: &Map) -> Result<Graph, Vec<Diagnostic>> {
    let mut c = Compiler {
        g: Graph::new(),
        errors: Vec::new(),
        edge_counts: HashMap::new(),
        warned: Vec::new(),
    };
    c.map(0, map);
    if c.errors.is_empty() {
        Ok(c.g)
    } else {
        Err(c.errors)
    }
}

struct Compiler {
    g: Graph,
    errors: Vec<Diagnostic>,
    edge_counts: HashMap<String, usize>,
    warned: Vec<String>,
}

fn key_string(segs: &[String]) -> String {
    segs.iter()
        .map(|s| {
            if s.is_empty() || s.contains(['.', ':', ';', '{', '}', '"', '\'', '#']) {
                format!("\"{}\"", s.replace('"', "\\\""))
            } else {
                s.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(".")
}

impl Compiler {
    fn err(&mut self, pos: Pos, msg: impl Into<String>) {
        self.errors.push(Diagnostic::error(pos.line, pos.col, msg));
    }

    fn warn_once(&mut self, msg: String) {
        if !self.warned.contains(&msg) {
            self.warned.push(msg.clone());
            self.g.warnings.push(msg);
        }
    }

    fn map(&mut self, scope: usize, map: &Map) {
        for st in &map.stmts {
            self.stmt(scope, st);
        }
    }

    fn stmt(&mut self, scope: usize, st: &Stmt) {
        if st.is_edge() {
            self.edge_stmt(scope, st);
            return;
        }
        let path = &st.keys[0];
        let kw = path.iter().position(is_reserved);
        if let Some(s) = path.iter().find(|s| is_style_keyword(s)) {
            if !path.iter().any(|s| !s.quoted && s.text == "style") {
                let p = s.pos;
                let t = s.text.clone();
                self.err(p, format!("{t:?} must be style.{t}"));
                return;
            }
        }
        match kw {
            Some(k) => {
                let Some(obj) = self.resolve(scope, &path[..k], true) else {
                    return;
                };
                self.field(obj, &path[k..], st);
            }
            None => {
                let Some(obj) = self.resolve(scope, path, true) else {
                    return;
                };
                if let Some(v) = &st.value {
                    self.set_label(obj, v);
                }
                if let Some(m) = &st.map {
                    self.map(obj, m);
                }
            }
        }
    }

    fn set_label(&mut self, obj: usize, v: &Value) {
        if obj == 0 {
            self.warn_once("a label on the root board is ignored by the native backend".into());
            return;
        }
        self.g.objects[obj].label = Some(v.text.clone());
        self.g.objects[obj].label_block = v.block;
    }

    /// Find or create the object at `path` under `scope`.
    fn resolve(&mut self, scope: usize, path: &[Seg], create: bool) -> Option<usize> {
        let mut cur = scope;
        for seg in path {
            if !seg.quoted && seg.text == "_" {
                if cur == 0 {
                    self.err(seg.pos, "parent \"_\" cannot be used in the root scope");
                    return None;
                }
                cur = self.g.objects[cur].parent;
                continue;
            }
            if is_reserved(seg) || is_style_keyword(seg) {
                self.err(
                    seg.pos,
                    format!("reserved keywords are prohibited in edges: {:?}", seg.text),
                );
                return None;
            }
            let found = self.g.objects[cur]
                .children
                .iter()
                .copied()
                .find(|&c| self.g.objects[c].id.eq_ignore_ascii_case(&seg.text));
            cur = match found {
                Some(c) => c,
                None if create => {
                    let parent = &self.g.objects[cur];
                    let mut segs: Vec<String> = Vec::new();
                    let mut i = cur;
                    while i != 0 {
                        segs.push(self.g.objects[i].id.clone());
                        i = self.g.objects[i].parent;
                    }
                    segs.reverse();
                    segs.push(seg.text.clone());
                    let level = parent.level + 1;
                    let idx = self.g.objects.len();
                    self.g.objects.push(Object {
                        id: seg.text.clone(),
                        abs: key_string(&segs),
                        parent: cur,
                        children: Vec::new(),
                        label: None,
                        label_block: false,
                        shape: "rectangle".into(),
                        style: Style::default(),
                        direction: None,
                        width: None,
                        height: None,
                        level,
                    });
                    self.g.objects[cur].children.push(idx);
                    idx
                }
                None => return None,
            };
        }
        Some(cur)
    }

    /// Apply a keyword path (`shape`, `style.fill`, ...) to `obj`.
    fn field(&mut self, obj: usize, path: &[Seg], st: &Stmt) {
        let kw = &path[0];
        let rest = &path[1..];
        let value = st.value.as_ref();
        match kw.text.as_str() {
            "style" => self.style_field(Target::Object(obj), rest, st),
            "label" => {
                if let Some(v) = value {
                    self.set_label(obj, v);
                }
                if st.map.is_some() || !rest.is_empty() {
                    self.warn_once("label positioning is ignored by the native backend".into());
                }
            }
            "shape" => {
                let Some(v) = value else {
                    return self.err(kw.pos, "shape must be a value");
                };
                let s = v.text.to_ascii_lowercase();
                if !SHAPES.contains(&s.as_str()) {
                    return self.err(v.pos, format!("unknown shape {:?}", v.text));
                }
                if !NATIVE_SHAPES.contains(&s.as_str()) {
                    self.warn_once(format!(
                        "shape {s:?} is drawn as a rectangle by the native backend"
                    ));
                }
                self.g.objects[obj].shape = s;
            }
            "direction" => {
                let Some(v) = value else {
                    return self.err(kw.pos, "direction must be a value");
                };
                let d = match v.text.as_str() {
                    "down" => Direction::Down,
                    "up" => Direction::Up,
                    "right" => Direction::Right,
                    "left" => Direction::Left,
                    other => {
                        return self.err(
                            v.pos,
                            format!(
                                "direction must be one of up, down, right, left, got {other:?}"
                            ),
                        )
                    }
                };
                self.g.objects[obj].direction = Some(d);
            }
            "width" | "height" => {
                let Some(v) = value else {
                    return self.err(kw.pos, format!("{} must be a value", kw.text));
                };
                match v.text.parse::<f64>() {
                    Ok(n) if n > 0.0 => {
                        if kw.text == "width" {
                            self.g.objects[obj].width = Some(n);
                        } else {
                            self.g.objects[obj].height = Some(n);
                        }
                    }
                    _ => self.err(
                        v.pos,
                        format!("expected {:?} to be a positive number", kw.text),
                    ),
                }
            }
            other => {
                self.warn_once(format!(
                    "{other:?} is not supported by the native backend and was ignored"
                ));
            }
        }
    }

    fn style_field(&mut self, target: Target, rest: &[Seg], st: &Stmt) {
        if let Some(k) = rest.first() {
            if rest.len() > 1 {
                return self.err(rest[1].pos, "style keywords cannot have children");
            }
            match &st.value {
                Some(v) => self.style_value(target, k, v),
                None => self.err(k.pos, format!("style.{} must be a value", k.text)),
            }
            return;
        }
        let Some(m) = &st.map else {
            return self.err(st.pos, "style must be a map");
        };
        for inner in &m.stmts {
            if inner.is_edge() || inner.keys[0].len() != 1 {
                self.err(inner.pos, "style map entries must be single keywords");
                continue;
            }
            match &inner.value {
                Some(v) => self.style_value(target, &inner.keys[0][0], v),
                None => self.err(inner.pos, "style keywords must have a value"),
            }
        }
    }

    fn style_value(&mut self, target: Target, k: &Seg, v: &Value) {
        if !is_style_keyword(k) {
            return self.err(k.pos, format!("invalid style keyword: {:?}", k.text));
        }
        let num = |lo: f64, hi: f64| -> Option<f64> {
            v.text.parse::<f64>().ok().filter(|n| *n >= lo && *n <= hi)
        };
        let boolean = || match v.text.as_str() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        };
        let style = match target {
            Target::Object(i) => &mut self.g.objects[i].style,
            Target::Edge(i) => &mut self.g.edges[i].style,
        };
        let name = k.text.as_str();
        let bad: Option<String> = match name {
            "fill" => {
                style.fill = Some(v.text.clone());
                None
            }
            "stroke" => {
                style.stroke = Some(v.text.clone());
                None
            }
            "font-color" => {
                style.font_color = Some(v.text.clone());
                None
            }
            "opacity" => match num(0.0, 1.0) {
                Some(n) => {
                    style.opacity = Some(n);
                    None
                }
                None => Some("expected \"opacity\" to be a number between 0.0 and 1.0".into()),
            },
            "stroke-width" => match num(0.0, 15.0) {
                Some(n) => {
                    style.stroke_width = Some(n);
                    None
                }
                None => Some("expected \"stroke-width\" to be a number between 0 and 15".into()),
            },
            "stroke-dash" => match num(0.0, 10.0) {
                Some(n) => {
                    style.stroke_dash = Some(n);
                    None
                }
                None => Some("expected \"stroke-dash\" to be a number between 0 and 10".into()),
            },
            "border-radius" => match num(0.0, f64::MAX) {
                Some(n) => {
                    style.border_radius = Some(n);
                    None
                }
                None => {
                    Some("expected \"border-radius\" to be a number greater or equal to 0".into())
                }
            },
            "font-size" => match num(8.0, 100.0) {
                Some(n) => {
                    style.font_size = Some(n);
                    None
                }
                None => Some("expected \"font-size\" to be a number between 8 and 100".into()),
            },
            "bold" | "italic" | "underline" | "shadow" | "multiple" | "double-border"
            | "animated" | "3d" | "filled" => match boolean() {
                Some(b) => {
                    match name {
                        "bold" => style.bold = Some(b),
                        "italic" => style.italic = Some(b),
                        "underline" => style.underline = Some(b),
                        "shadow" => style.shadow = Some(b),
                        "multiple" => style.multiple = Some(b),
                        "double-border" => style.double_border = Some(b),
                        "animated" => style.animated = Some(b),
                        _ => {}
                    }
                    None
                }
                None => Some(format!("expected {name:?} to be true or false")),
            },
            _ => {
                let msg =
                    format!("style.{name} is not supported by the native backend and was ignored");
                self.warn_once(msg);
                None
            }
        };
        if let Some(msg) = bad {
            self.err(v.pos, msg);
        }
    }

    fn edge_stmt(&mut self, scope: usize, st: &Stmt) {
        let mut ends = Vec::new();
        for k in &st.keys {
            match self.resolve(scope, k, true) {
                Some(o) => ends.push(o),
                None => return,
            }
        }
        if ends.iter().any(|&e| e == 0) {
            return self.err(st.pos, "connections cannot target the root board");
        }
        let scope_key = self.g.objects[scope].abs.clone();
        let rel = |c: &Compiler, o: usize| -> String {
            let abs = &c.g.objects[o].abs;
            if scope_key.is_empty() {
                abs.clone()
            } else {
                abs.strip_prefix(&format!("{scope_key}."))
                    .unwrap_or(abs)
                    .to_string()
            }
        };
        for (i, op) in st.ops.iter().enumerate() {
            let (a, b) = (ends[i], ends[i + 1]);
            let base = format!("({} {} {})", rel(self, a), op.as_str(), rel(self, b));
            let full_base = if scope_key.is_empty() {
                base
            } else {
                format!("{scope_key}.{base}")
            };
            let n = self.edge_counts.entry(full_base.clone()).or_insert(0);
            let key = format!("{full_base}[{n}]");
            *n += 1;
            let (src, dst, src_arrow, dst_arrow) = match op {
                EdgeOp::Forward => (a, b, false, true),
                EdgeOp::Backward => (a, b, true, false),
                EdgeOp::Both => (a, b, true, true),
                EdgeOp::Undirected => (a, b, false, false),
            };
            self.g.edges.push(Edge {
                src,
                dst,
                src_arrow,
                dst_arrow,
                label: st.value.as_ref().map(|v| v.text.clone()),
                style: Style::default(),
                key,
            });
            let ei = self.g.edges.len() - 1;
            if let Some(m) = &st.map {
                self.edge_map(ei, m);
            }
        }
    }

    fn edge_map(&mut self, ei: usize, m: &Map) {
        for inner in &m.stmts {
            if inner.is_edge() {
                self.err(
                    inner.pos,
                    "connections cannot be declared inside a connection",
                );
                continue;
            }
            let path: &KeyPath = &inner.keys[0];
            let head = &path[0];
            match (head.quoted, head.text.as_str()) {
                (false, "label") => {
                    if let Some(v) = &inner.value {
                        self.g.edges[ei].label = Some(v.text.clone());
                    }
                }
                (false, "style") => self.style_field(Target::Edge(ei), &path[1..], inner),
                (false, "source-arrowhead" | "target-arrowhead") => {
                    self.warn_once(
                        "arrowhead customisation is ignored by the native backend".into(),
                    );
                }
                _ => {
                    self.warn_once(format!(
                        "{:?} on a connection is ignored by the native backend",
                        head.text
                    ));
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Target {
    Object(usize),
    Edge(usize),
}

#[cfg(test)]
mod tests {
    use super::super::parser::parse;
    use super::*;

    fn g(src: &str) -> Graph {
        compile(&parse(src).unwrap()).unwrap()
    }

    #[test]
    fn objects_and_edges() {
        let g = g("a: Alpha\ngrp: {\n  c\n  c -> d\n}\na -> grp.c\na -> grp.c\n");
        let abs: Vec<_> = g.objects.iter().skip(1).map(|o| o.abs.as_str()).collect();
        assert_eq!(abs, ["a", "grp", "grp.c", "grp.d"]);
        let keys: Vec<_> = g.edges.iter().map(|e| e.key.as_str()).collect();
        assert_eq!(
            keys,
            ["grp.(c -> d)[0]", "(a -> grp.c)[0]", "(a -> grp.c)[1]"]
        );
        assert_eq!(g.objects[1].label.as_deref(), Some("Alpha"));
    }

    #[test]
    fn errors() {
        let e = compile(&parse("x: {shape: nosuch}\ny.style.opacity: 7\n").unwrap()).unwrap_err();
        let msgs: Vec<_> = e.iter().map(|d| d.to_string()).collect();
        assert_eq!(
            msgs,
            [
                "1:12: unknown shape \"nosuch\"",
                "2:18: expected \"opacity\" to be a number between 0.0 and 1.0"
            ]
        );
    }

    #[test]
    fn styles_and_shapes() {
        let g = g("a.shape: circle\na.style: {fill: red; stroke-width: 3}\nb -> a: {style.stroke: blue}\n");
        let a = &g.objects[1];
        assert_eq!(a.shape, "circle");
        assert_eq!(a.style.fill.as_deref(), Some("red"));
        assert_eq!(a.style.stroke_width, Some(3.0));
        assert_eq!(g.edges[0].style.stroke.as_deref(), Some("blue"));
    }
}
