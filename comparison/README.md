# Comparison Projects - DISABLED

This directory contains comparison benchmarks between different Rust session type libraries:
- Rumpsteak
- Ferrite
- mpstthree
- Sesh

## Why Disabled

These comparison projects are currently disabled in the Aura fork because they depend on libraries that require nightly Rust features, specifically:

- `sesh` library uses `#![feature(never_type)]` which is not available in stable Rust 1.90
- Other dependencies may have similar nightly requirements

## Re-enabling

To re-enable these comparison projects:

1. Switch to nightly Rust toolchain
2. Add `"comparison"` back to the workspace members in the root `Cargo.toml`
3. Run `cargo build --all` in the comparison directory

## Alternative

For Aura development purposes, the core rumpsteak-aura library and its examples provide sufficient testing and validation without needing cross-library comparisons.