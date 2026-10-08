# llm-anthropic

Anthropic Messages API on top of `llm-wire` and `llm-http`. System turns are sent in the `system` field. The caller injects the transport. This crate does not open a socket, and errors do not include the API key.
