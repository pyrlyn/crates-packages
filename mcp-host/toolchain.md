# Toolchain

| Program | How to install | Why here | Source |
| --- | --- | --- | --- |
| rustc | mise | Build | https://github.com/rust-lang/rust |
| just | mise | Test recipe | https://github.com/casey/just |
| ketch | see its README | Installs swarfr | https://github.com/pyrlyn/ketch |
| swarfr | ketch | Lossless cleanup of target/ after tests | https://github.com/listepo/swarfr |

## cargo

| Package | Where | Source | Why here |
| --- | --- | --- | --- |
| rmcp | local | https://github.com/modelcontextprotocol/rust-sdk | The MCP protocol, client transports and the OAuth flow |
| async-trait | local | https://github.com/dtolnay/async-trait | `CredentialStore` implementations (rmcp's trait is async-trait) |
| tokio | local | https://github.com/tokio-rs/tokio | Loopback OAuth callback listener, timeouts, blocking keyring calls |
| keyring | local, optional (`keyring` feature) | https://github.com/open-source-cooperative/keyring-rs | OS keychain as a token store |
| serde_json | local | https://github.com/serde-rs/json | Reading `.mcp.json` and `~/.claude.json`; JSON values of tool input and output |
| tempfile | local | https://github.com/Stebalien/tempfile | Scratch project and home trees in discovery tests |

## ketch

| Package | Where | Source | Why here |
| --- | --- | --- | --- |
| swarfr | global | https://github.com/listepo/swarfr | Lossless cleanup of target/ after tests |
