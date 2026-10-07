# wasm-plugin-host

Run WebAssembly plugins with [extism](https://github.com/extism/extism), the
part every plugin host repeats:

- each plugin on a worker thread of its own, so a slow plugin never holds the
  application's threads and calls into one plugin never interleave;
- a control lane (calls the application waits on) served before an event lane,
  each with a bounded queue that answers `Busy` when full;
- a per-call deadline enforced by a watchdog, and a linear-memory cap, both
  reported as typed errors the caller fails open on;
- no WASI and no extism HTTP: everything a plugin may do goes through host
  functions the application grants.

```rust
use std::sync::Arc;
use wasm_plugin_host::{HostEnv, Lane, Limits, Options, PluginHost, extism::Function};

struct Env;

impl HostEnv for Env {
    fn id(&self) -> &str { "demo" }
    fn functions(self: &Arc<Self>) -> Vec<Function> { Vec::new() }
}

const OPTIONS: Options = Options::new("app_init", "app-plugin");

let host = PluginHost::load(&wasm, &Limits::default(), &OPTIONS, Arc::new(Env))?;
let out: Option<serde_json::Value> =
    host.call(Lane::Control, "app_init", &serde_json::json!({}), host.call_cap())?;
```

Inputs and outputs are JSON. An export the module does not have answers
`Ok(None)`; only `Options::required_export` must exist.

Extracted from cox's `cox-plugin` host so cox and scull share one engine.

Licensed under either of MIT or Apache-2.0 at your option.
