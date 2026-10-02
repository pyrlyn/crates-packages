// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Colors of D2's "Neutral Default" theme and its fill rules (d2themes,
// d2themescatalog, d2graph GetFill/GetStroke), Copyright 2022 Terrastruct, Inc.

//! The default theme.

use super::graph::Graph;

/// Theme 0, "Neutral Default".
pub struct Theme {
    pub n1: &'static str,
    pub n2: &'static str,
    pub n4: &'static str,
    pub n5: &'static str,
    pub n7: &'static str,
    pub b1: &'static str,
    pub b2: &'static str,
    pub b3: &'static str,
    pub b4: &'static str,
    pub b5: &'static str,
    pub b6: &'static str,
    pub aa4: &'static str,
    pub aa5: &'static str,
    pub ab4: &'static str,
    pub ab5: &'static str,
}

/// The only theme the native backend draws.
pub const NEUTRAL_DEFAULT: Theme = Theme {
    n1: "#0A0F25",
    n2: "#676C7E",
    n4: "#CFD2DD",
    n5: "#DEE1EB",
    n7: "#FFFFFF",
    b1: "#0D32B2",
    b2: "#0D32B2",
    b3: "#E3E9FD",
    b4: "#E3E9FD",
    b5: "#EDF0FD",
    b6: "#F7F8FE",
    aa4: "#EDF0FD",
    aa5: "#F7F8FE",
    ab4: "#EDF0FD",
    ab5: "#F7F8FE",
};

impl Theme {
    /// Default fill of object `i`.
    pub fn fill(&self, g: &Graph, i: usize) -> &'static str {
        let o = &g.objects[i];
        let level = o.level;
        match o.shape.as_str() {
            "rectangle" | "square" | "circle" | "oval" | "hierarchy" => match level {
                1 if !o.is_container() => self.b6,
                1 => self.b4,
                2 => self.b5,
                3 => self.b6,
                _ => self.n7,
            },
            "cylinder" | "stored_data" | "package" => {
                if level == 1 {
                    self.aa4
                } else {
                    self.aa5
                }
            }
            "step" | "page" | "document" => {
                if level == 1 {
                    self.ab4
                } else {
                    self.ab5
                }
            }
            "person" | "c4-person" => self.b3,
            "diamond" => self.n4,
            "queue" | "parallelogram" | "hexagon" => self.n5,
            _ => self.n7,
        }
    }

    /// Default stroke of object `i`.
    pub fn stroke(&self, g: &Graph, i: usize, dashed: bool) -> &'static str {
        match g.objects[i].shape.as_str() {
            "text" | "code" => self.n1,
            _ if dashed => self.b2,
            _ => self.b1,
        }
    }
}
