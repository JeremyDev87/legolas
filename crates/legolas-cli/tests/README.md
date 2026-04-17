# legolas-cli tests

This directory is intentionally seeded in the foundation phase.

Current coverage:

- `cli_contract.rs` for Rust CLI help, version, scan, visualize, optimize, and validation-error contract checks against the checked-in oracle corpus

Planned follow-up tests:

- `text_report_parity.rs` for formatter-only parity once the Rust analysis pipeline no longer bridges through the JS source-of-truth path
