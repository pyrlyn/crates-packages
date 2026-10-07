//! What a wire needs to know about its section and its models, as plain
//! values. cox keeps these in `cox-protocol::config` and `cox-models`, which
//! carry its whole config schema and catalog; the wires read only the fields
//! below, so this crate takes them as small structs instead.

use llm_wire::Effort;

/// The transport knobs one provider section carries.
#[derive(Debug, Clone, PartialEq)]
pub struct Transport {
    /// API base URL.
    pub base_url: String,
    /// Env var holding the API key; the caller resolves it (a blank or unset
    /// key builds a keyless client with no `Authorization` header).
    pub api_key_env: String,
    /// Read/idle timeout, in seconds.
    pub timeout_s: u32,
    /// Max retries for retryable errors.
    pub max_retries: u32,
}

/// One entry of a section's `models` list, reduced to what the wires read.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProviderModel {
    /// The model id sent on the wire (for gateways the full `vendor/model`).
    pub id: String,
    /// Context window in tokens; `0` means not declared.
    pub context_window: u32,
    /// Whether this model takes the Chat `reasoning_effort` field. Unset
    /// means not declared, and the Chat wire then sends no effort at all:
    /// OpenAI documents the field but LM Studio's endpoint does not list it.
    pub reasoning_effort: Option<bool>,
    /// Whether this model takes image input. `false` makes the Chat wire
    /// refuse a request that carries an image; unset still sends it.
    pub images: Option<bool>,
}

/// What a model's `models` entry declares about its capabilities.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Capabilities {
    /// Accepts the Chat `reasoning_effort` field.
    pub reasoning_effort_param: Option<bool>,
    /// Accepts image input.
    pub images: Option<bool>,
}

impl Capabilities {
    /// What `model` declares.
    pub fn declared_by(model: &ProviderModel) -> Self {
        Self {
            reasoning_effort_param: model.reasoning_effort,
            images: model.images,
        }
    }

    /// The effort the Chat wire sends, or `None` when the model does not
    /// declare `reasoning_effort`. The Responses wire needs no row: it always
    /// sends the request's effort.
    pub fn chat_effort(&self, effort: Effort) -> Option<Effort> {
        (self.reasoning_effort_param == Some(true)).then_some(effort)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_sends_effort_only_when_the_model_declares_it() {
        for (declared, want) in [
            (None, None),
            (Some(false), None),
            (Some(true), Some(Effort::High)),
        ] {
            let caps = Capabilities {
                reasoning_effort_param: declared,
                images: None,
            };
            assert_eq!(
                caps.chat_effort(Effort::High),
                want,
                "declared {declared:?}"
            );
        }
    }

    #[test]
    fn declared_by_reads_the_model_entry() {
        let model = ProviderModel {
            id: "m".into(),
            reasoning_effort: Some(true),
            images: Some(false),
            ..Default::default()
        };
        assert_eq!(
            Capabilities::declared_by(&model),
            Capabilities {
                reasoning_effort_param: Some(true),
                images: Some(false)
            }
        );
    }
}
