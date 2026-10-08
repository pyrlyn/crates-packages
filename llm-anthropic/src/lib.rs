//! Anthropic Messages API.
//!
//! System turns move to the `system` field. The call goes through
//! [`llm_http::Transport`]. This crate does not open a socket, and the API
//! key is never copied into an error.

use llm_http::{Client, Error as HttpError, Pause, Transport};
use llm_wire::{ChatRequest, ChatResponse, Error as WireError, Role, Usage};
use serde::{Deserialize, Serialize};

const API_VERSION: &str = "2023-06-01";
const MAX_TOKENS: u32 = 1024;

/// Why a message call failed. The text never includes the API key or the response body.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The request has no messages.
    #[error(transparent)]
    Wire(#[from] WireError),
    /// The transport or the HTTP status failed.
    #[error(transparent)]
    Http(#[from] HttpError),
    /// Anthropic needs a user or assistant turn after system text is split out.
    #[error("Anthropic needs a user or assistant turn")]
    Turns,
    /// The JSON was not a messages response.
    #[error("Anthropic response is missing text")]
    Shape,
}

/// Where to send the call.
pub struct Endpoint<'a> {
    /// API root, without a trailing slash.
    pub base: &'a str,
    /// `x-api-key` value. Not written to errors.
    pub key: &'a str,
}

#[derive(Serialize)]
struct WireMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct WireRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<WireMessage<'a>>,
}

#[derive(Deserialize)]
struct WireResponse {
    content: Vec<Block>,
    #[serde(default)]
    usage: Option<WireUsage>,
}

#[derive(Deserialize)]
struct Block {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Deserialize)]
struct WireUsage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
}

/// POST `{base}/messages`.
///
/// # Errors
///
/// [`Error::Wire`] when the request is empty.
/// [`Error::Turns`] when every message is a system turn.
/// [`Error::Http`] when the transport fails or the status is not success.
/// [`Error::Shape`] when the body has no text block.
pub fn complete<T, P>(
    client: &Client<T, P>,
    endpoint: &Endpoint<'_>,
    request: &ChatRequest,
) -> Result<ChatResponse, Error>
where
    T: Transport,
    P: Pause,
{
    request.validate()?;
    let system = system_text(request);
    let messages = conversation(request);
    if messages.is_empty() {
        return Err(Error::Turns);
    }
    let body = WireRequest {
        model: &request.model,
        max_tokens: MAX_TOKENS,
        system,
        messages,
    };
    let bytes = serde_json::to_vec(&body).map_err(|_| Error::Shape)?;
    let url = format!("{}/messages", endpoint.base.trim_end_matches('/'));
    let headers = vec![
        ("x-api-key".to_owned(), endpoint.key.to_owned()),
        ("anthropic-version".to_owned(), API_VERSION.to_owned()),
        ("content-type".to_owned(), "application/json".to_owned()),
    ];
    let response = client.post(&url, &headers, &bytes)?;
    let parsed: WireResponse = serde_json::from_slice(&response.body).map_err(|_| Error::Shape)?;
    let text = parsed
        .content
        .into_iter()
        .filter(|block| block.kind == "text")
        .filter_map(|block| block.text)
        .collect::<Vec<_>>()
        .join("");
    if text.is_empty() {
        return Err(Error::Shape);
    }
    let usage = parsed.usage.map_or(Usage::default(), |usage| Usage {
        input_tokens: usage.input_tokens,
        output_tokens: usage.output_tokens,
    });
    Ok(ChatResponse { text, usage })
}

fn system_text(request: &ChatRequest) -> Option<String> {
    let text = request
        .messages
        .iter()
        .filter(|message| message.role == Role::System)
        .map(|message| message.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn conversation(request: &ChatRequest) -> Vec<WireMessage<'_>> {
    request
        .messages
        .iter()
        .filter(|message| message.role != Role::System)
        .map(|message| WireMessage {
            role: match message.role {
                Role::Assistant => "assistant",
                Role::User | Role::System => "user",
            },
            content: &message.text,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use llm_http::{HttpRequest, HttpResponse};
    use llm_wire::Message;

    struct Script;

    impl Transport for Script {
        fn execute(&self, request: &HttpRequest) -> Result<HttpResponse, HttpError> {
            assert_eq!(request.url, "http://example.test/v1/messages");
            assert!(request
                .headers
                .iter()
                .any(|(name, value)| name == "x-api-key" && value == "fixture-key"));
            let payload: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
            assert_eq!(payload["system"], "be brief");
            assert_eq!(payload["messages"][0]["role"], "user");
            let body = br#"{"content":[{"type":"text","text":"hello"}],"usage":{"input_tokens":2,"output_tokens":1}}"#;
            Ok(HttpResponse {
                status: 200,
                body: body.to_vec(),
            })
        }
    }

    #[test]
    fn system_text_is_split_out_and_the_reply_is_mapped() {
        let request = ChatRequest {
            model: "m".into(),
            messages: vec![
                Message {
                    role: Role::System,
                    text: "be brief".into(),
                },
                Message {
                    role: Role::User,
                    text: "hi".into(),
                },
            ],
        };
        let response = complete(
            &Client::new(Script),
            &Endpoint {
                base: "http://example.test/v1",
                key: "fixture-key",
            },
            &request,
        )
        .unwrap();
        assert_eq!(response.text, "hello");
        assert_eq!(response.usage.output_tokens, 1);
    }
}
