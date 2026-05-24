# tensor_vm_explorer

Standalone browser explorer for TensorVM.

The crate owns the default terminal-style explorer UI shell, a Ratzilla/Ratatui WASM entry point, and
neutral JSON view models. It does not embed chain state. At runtime `tensorvm-explorer` serves the
page and the page polls a TensorVM node through the node's `/explorer/ws` WebSocket endpoint.

Default settings:

```text
TENSORVM_EXPLORER_LISTEN=127.0.0.1:8080
TENSORVM_EXPLORER_WS_URL=ws://127.0.0.1:8545/explorer/ws
```

Run the HTTP explorer server:

```bash
cargo run -p tensor_vm_explorer -- serve
```

Override defaults with flags or the environment:

```bash
cargo run -p tensor_vm_explorer -- serve \
  --listen 127.0.0.1:8080 \
  --ws-url ws://127.0.0.1:8545/explorer/ws
```

Check a running server:

```bash
cargo run -p tensor_vm_explorer -- health-check --addr 127.0.0.1:8080
```

In the local CPU Compose testnet the explorer is available at `http://127.0.0.1:8080` and polls
`miner-00` through `ws://127.0.0.1:8545/explorer/ws?token=local-cpu-testnet-token`.

Run from the workspace root:

```bash
cargo test -p tensor_vm_explorer --release
```

The `ui` feature is enabled by default. A browser/WASM Ratzilla build can be checked with:

```bash
cargo check -p tensor_vm_explorer --target wasm32-unknown-unknown --bin tensorvm-explorer-ratzilla
```
