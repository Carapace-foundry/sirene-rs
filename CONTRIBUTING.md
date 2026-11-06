# Contributing to sirene-rs

Thank you for your interest in contributing to **sirene-rs**! ⚡

`sirene-rs` is a deterministic, ultra-fast French SIRENE / RNE corporate record linkage and search engine built in pure Rust and WebAssembly.

---

## Development Setup

1. **Prerequisites**:
   - Modern Rust toolchain (Rust 1.80+):
     ```bash
     rustup update stable
     rustup target add wasm32-unknown-unknown
     ```

2. **Clone the repository**:
   ```bash
   git clone https://github.com/carapace-foundry/sirene-rs.git
   cd sirene-rs
   ```

3. **Build the workspace**:
   ```bash
   cargo build
   ```

4. **Run the test suite**:
   ```bash
   cargo test --workspace --all-targets
   ```

5. **Verify WebAssembly compilation (zero native C dependency rule)**:
   ```bash
   cargo check --target wasm32-unknown-unknown -p sirene-wasm
   ```

6. **Check code formatting and linter**:
   ```bash
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets -- -D warnings
   ```

---

## Architectural Principles

Before submitting changes, ensure your code aligns with the repository's core guidelines:
- **Zero C / C++ Dependencies**: No `gcc`, `clang`, or dynamic system C libraries. Everything must compile to `wasm32-unknown-unknown`.
- **Zero Panic in Production Crates**: Never use `unwrap()` or `expect()` inside `sirene-core`, `sirene-normalize`, `sirene-index`, or `sirene-matcher`. Use `thiserror` for typed errors.
- **Minimal / Zero Allocations**: Leverage borrowed slices (`&str`, `&[u8]`) and `SmallVec` when possible.
- **Ground Truth Compliance**: Preserve strict adherence to official French administrative standards (Luhn verification, INSEE COG communes, NAF Rev. 2, and DINUM legal abbreviations).

---

## Pull Requests

- Keep PRs focused on a single topic, bug fix, or feature.
- Ensure all automated tests (`cargo test --workspace --all-targets`) pass cleanly.
- Follow Rust standard formatting (`cargo fmt`).
- Ensure `cargo clippy --workspace --all-targets -- -D warnings` reports zero warnings.
- Update documentation in `docs/` or `README.md` whenever adding or changing functionality.
