# Motley

Motley is a modular Rust game engine project organized as a Cargo workspace.

## Prerequisites

- Rust stable toolchain
- Git
- Platform linker toolchain

### Windows
Install one of the following:
- Visual Studio Build Tools 2022 with "Desktop development with C++"
- Visual Studio Community with C++ workload and Windows SDK

## Quick start

```bash
cargo build --workspace
cargo run -p sandbox
```

## Quality checks

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
