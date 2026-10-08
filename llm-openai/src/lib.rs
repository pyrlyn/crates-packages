//! OpenAI chat completions.
//!
//! The HTTP call goes through [`llm_http::Transport`]. This crate does not
//! open a socket. The API key is a header value and is never copied into an error.

use llm_http::{Client, Error as HttpError, Pause, Transport};
use llm_wire::{ChatRequest, ChatResponse, Error as WireError, Message, Role, Usage};
use serde::Deserialize;
use serde::Serialize;

/// Why a chat completion failed. The text never includes the API key or the response body.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The request has no messages.
    #[error(transparent)]
    Wire(#[from] WireError),
    /// The transport or the HTTP status failed.
    #[error(transparent)]
    Http(#[from] HttpError),
    /// The JSON was not a chat completion.
    #[error("OpenAI response is missing a message")]
    Shape,
}

/// Where to send the call.
pub struct Endpoint<'a> {
    /// API root, without a trailing slash, such as the server the caller configured.
    pub base: &'a str,
    /// Bearer token. Not written to errors.
    pub key: &'a str,
}

#[derive(Serialize)]
struct WireMessage<'a> {
    role: Role,
    content: &'a str,
}

#[derive(Serialize)]
struct WireRequest<'a> {
    model: &'a str,
    messages: Vec<WireMessage<'a>>,
}

#[derive(Deserialize)]
struct WireResponse {
    choices: Vec<Choice>,
    #[serde(default)]
    usage: Option<WireUsage>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(Deserialize)]
struct ChoiceMessage {
    content: String,
}

#[derive(Deserialize)]
struct WireUsage {
    #[serde(default)]
    prompt_tokens: u64,
    #[serde(default)]
    completion_tokens: u64,
}

/// POST `{base}/chat/completions` and map the first choice.
///
/// # Errors
///
/// [`Error::Wire`] when the request is empty.
/// [`Error::Http`] when the transport fails or the status is not success.
/// [`Error::Shape`] when the body is not a chat completion.
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
    let body = WireRequest {
        model: &request.model,
        messages: request
            .messages
            .iter()
            .map(|message| WireMessage {
                role: message.role,
                content: &message.text,
            })
            .collect(),
    };
    let bytes = serde_json::to_vec(&body).map_err(|_| Error::Shape)?;
    let url = format!("{}/chat/completions", endpoint.base.trim_end_matches('/'));
    let headers = vec![
        (
            "authorization".to_owned(),
            format!("Bearer {}", endpoint.key),
        ),
        ("content-type".to_owned(), "application/json".to_owned()),
    ];
    let response = client.post(&url, &headers, &bytes)?;
    let parsed: WireResponse = serde_json::from_slice(&response.body).map_err(|_| Error::Shape)?;
    let text = parsed
        .choices
        .into_iter()
        .next()
        .map(|choice| choice.message.content)
        .ok_or(Error::Shape)?;
    let usage = parsed.usage.map_or(Usage::default(), |usage| Usage {
        input_tokens: usage.prompt_tokens,
        output_tokens: usage.completion_tokens,
    });
    Ok(ChatResponse { text, usage })
}

/// A single user turn, for callers that do not build the message list themselves.
#[must_use]
pub fn user(model: &str, text: &str) -> ChatRequest {
    ChatRequest {
        model: model.to_owned(),
        messages: vec![Message {
            role: Role::User,
            text: text.to_owned(),
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use llm_http::{HttpRequest, HttpResponse};

    struct Script;

    impl Transport for Script {
        fn execute(&self, request: &HttpRequest) -> Result<HttpResponse, HttpError> {
            assert_eq!(request.url, "http://example.test/v1/chat/completions");
            assert!(request
                .headers
                .iter()
                .any(|(name, value)| name == "authorization" && value == "Bearer fixture-key"));
            let body = br#"{"choices":[{"message":{"role":"assistant","content":"hello"}}],"usage":{"prompt_tokens":3,"completion_tokens":1}}"#;
            Ok(HttpResponse {
                status: 200,
                body: body.to_vec(),
            })
        }
    }

    #[test]
    fn a_scripted_completion_maps_the_first_choice() {
        let client = Client::new(Script);
        let response = complete(
            &client,
            &Endpoint {
                base: "http://example.test/v1",
                key: "fixture-key",
            },
            &user("m", "hi"),
        )
        .unwrap();
        assert_eq!(response.text, "hello");
        assert_eq!(response.usage.input_tokens, 3);
        assert_eq!(response.usage.output_tokens, 1);
    }

    #[test]
    fn a_bad_body_is_a_shape_error_without_the_key() {
        struct Bad;
        impl Transport for Bad {
            fn execute(&self, _: &HttpRequest) -> Result<HttpResponse, HttpError> {
                Ok(HttpResponse {
                    status: 200,
                    body: b"{}".to_vec(),
                })
            }
        }
        let err = complete(
            &Client::new(Bad),
            &Endpoint {
                base: "http://example.test/v1",
                key: "fixture-key",
            },
            &user("m", "hi"),
        )
        .unwrap_err();
        let text = err.to_string();
        assert!(!text.contains("fixture-key"));
        assert!(matches!(err, Error::Shape));
    }
}
