# Vendor Layout

Rust release binaries are staged under `vendor/<triple>/legolas[.exe]`.

Examples:

- `vendor/x86_64-unknown-linux-gnu/legolas`
- `vendor/x86_64-pc-windows-msvc/legolas.exe`
- `vendor/x86_64-apple-darwin/legolas`
- `vendor/aarch64-apple-darwin/legolas`

This directory is intentionally kept empty in git except for this README.
Release workflows populate it transiently before uploading release assets, and
the npm launcher uses the same layout when packaging platform binaries.

Local staging flow:

```bash
cargo build --release -p legolas-cli
node ./scripts/stage-local-vendor-binary.mjs --vendor-dir /tmp/legolas-vendor
node ./scripts/verify-vendor-layout.mjs --vendor-dir /tmp/legolas-vendor
```
