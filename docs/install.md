# Installation

`sirene-rs` is distributed as a standalone Rust crate workspace, a native CLI binary (`sirene`), a Docker container, and a WebAssembly package (`sirene-wasm`).

---

## 🛠️ Building from Source

### Prerequisites

You need a modern Rust toolchain (version 1.80 or later):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update stable
```

If you plan to compile WebAssembly bindings:
```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
```

### Build CLI

```bash
git clone https://github.com/carapace-foundry/sirene-rs.git
cd sirene-rs

# Build optimized release binary
cargo build --release --bin sirene

# Copy binary to system path
sudo cp target/release/sirene /usr/local/bin/
```

Verify installation:
```bash
sirene --version
```

---

## 🐳 Docker Container

You can run `sirene-rs` in a secure, non-root container:

```bash
# Pull from GitHub Container Registry
docker pull ghcr.io/carapace-foundry/sirene-rs:latest

# Or build locally
docker build -t sirene-rs:latest .
```

---

## 📦 WebAssembly Build

```bash
cd crates/sirene-wasm
wasm-pack build --target web --release
```

This generates the NPM-compatible `pkg/` folder with TypeScript definitions (`.d.ts`) and WebAssembly modules.
