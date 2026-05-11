---
bump: patch
---

Bump minimum Rust version to 1.74 (KAN-420)

- `CONTRIBUTING.md` prerequisites now correctly state Rust 1.74+ instead of 1.70+
- `rust-version = "1.74"` added to the workspace `Cargo.toml` so `cargo` enforces the minimum at build time
- The actual floor is set by `ratatui 0.29` and `clap 4.5`, both of which declare a 1.74 MSRV
