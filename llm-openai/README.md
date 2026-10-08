# llm-openai

OpenAI chat completions on top of `llm-wire` and `llm-http`. The caller injects the transport. This crate does not open a socket, and errors do not include the API key.
