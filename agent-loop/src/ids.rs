//! ULID-backed ids for what the loop itself names: a turn (`TurnId`) and a
//! transcript item (`ItemId`). Tool calls reuse `llm_wire::CallId`, which
//! the provider stream already carries. Newtypes, so a mixed-up id is a
//! compile error; each serializes as its ULID string.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// Declares a ULID newtype with `new`, `Display`, `FromStr` and
/// string-shaped serde, so both ids behave the same.
macro_rules! ulid_id {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(Ulid);

        impl $name {
            /// Generates a fresh, time-sortable id.
            pub fn new() -> Self {
                Self(Ulid::generate())
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, f)
            }
        }

        impl FromStr for $name {
            type Err = ulid::DecodeError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(Ulid::from_str(s)?))
            }
        }
    };
}

ulid_id!(
    TurnId,
    "Identifies one user turn, from `TurnStarted` to `TurnDone`."
);
ulid_id!(
    ItemId,
    "Identifies one transcript item, from `ItemStarted` to `ItemDone`."
);

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn ids_roundtrip_through_display_and_json() {
        let id = TurnId::new();
        let parsed: TurnId = id.to_string().parse().expect("display form parses back");
        assert_eq!(id, parsed);
        let json = serde_json::to_string(&id).expect("serialize");
        assert_eq!(json, format!("\"{id}\""));
    }
}
