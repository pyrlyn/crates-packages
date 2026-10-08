//! Provider-neutral chat messages and server-sent events.
//!
//! Providers map these types onto their own JSON. Nothing here opens a socket.

use serde::{Deserialize, Serialize};

/// Why a chat request cannot be sent.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    /// The request has no messages.
    #[error("a chat request needs at least one message")]
    Empty,
}

/// Who wrote a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Instructions that sit above the conversation.
    System,
    /// The person using the client.
    User,
    /// The model.
    Assistant,
}

/// One turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    /// Who wrote it.
    pub role: Role,
    /// Plain text. Callers sanitize untrusted text before it is shown.
    pub text: String,
}

/// A chat completion request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatRequest {
    /// Model id the provider should use.
    pub model: String,
    /// Turns, in order.
    pub messages: Vec<Message>,
}

impl ChatRequest {
    /// Rejects a request that has nothing to send.
    ///
    /// # Errors
    ///
    /// [`Error::Empty`] when `messages` is empty.
    pub fn validate(&self) -> Result<(), Error> {
        if self.messages.is_empty() {
            Err(Error::Empty)
        } else {
            Ok(())
        }
    }
}

/// Token counts reported by a provider. Absent counts stay zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Usage {
    /// Prompt tokens.
    pub input_tokens: u64,
    /// Completion tokens.
    pub output_tokens: u64,
}

/// A completed reply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatResponse {
    /// Assistant text.
    pub text: String,
    /// Usage, when the provider sent it.
    pub usage: Usage,
}

/// One server-sent event. `data` lines are joined with newlines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseEvent {
    /// The `event:` field, when the server set one.
    pub event: Option<String>,
    /// Joined `data:` lines, without a trailing newline.
    pub data: String,
}

/// Parses an SSE byte stream already decoded as text.
///
/// A blank line ends an event. Comment lines (starting with `:`) are skipped.
/// An event with no `data` is dropped.
#[must_use]
pub fn parse_sse(input: &str) -> Vec<SseEvent> {
    let mut events = Vec::new();
    let mut event = None;
    let mut data: Vec<String> = Vec::new();
    for line in input.split('\n') {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            push_event(&mut events, &mut event, &mut data);
            continue;
        }
        if line.starts_with(':') {
            continue;
        }
        if let Some(name) = line.strip_prefix("event:") {
            event = Some(name.trim_start().to_owned());
        } else if let Some(chunk) = line.strip_prefix("data:") {
            data.push(chunk.trim_start().to_owned());
        }
    }
    push_event(&mut events, &mut event, &mut data);
    events
}

fn push_event(events: &mut Vec<SseEvent>, event: &mut Option<String>, data: &mut Vec<String>) {
    if data.is_empty() {
        *event = None;
        return;
    }
    events.push(SseEvent {
        event: event.take(),
        data: data.join("\n"),
    });
    data.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sse_joins_data_lines_and_skips_comments() {
        let input = ": keep-alive\nevent: delta\ndata: hello\ndata: world\n\ndata: [DONE]\n";
        let events = parse_sse(input);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event.as_deref(), Some("delta"));
        assert_eq!(events[0].data, "hello\nworld");
        assert_eq!(events[1].data, "[DONE]");
    }

    #[test]
    fn an_empty_request_is_rejected() {
        let request = ChatRequest {
            model: "m".into(),
            messages: Vec::new(),
        };
        assert_eq!(request.validate(), Err(Error::Empty));
    }
}
