# llm-testkit

Replay recorded LLM calls. Request and response bodies are redacted with `telemetry-setup` before they are stored, so a cassette does not keep a bearer token. `Cassette` implements `llm-http`'s `Transport` and does not open a socket.
