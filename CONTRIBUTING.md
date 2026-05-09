# Contributing to Legolas

Thank you for your interest in contributing to Legolas.

## Before You Start

- Read the [Code of Conduct](./CODE_OF_CONDUCT.md)
- Check existing [issues](https://github.com/JeremyDev87/legolas/issues)
- Open an issue first for larger feature proposals or design changes

## Development Setup

### Prerequisites

- Node.js 18.17 or newer
- Rust 1.95.0 with `cargo`

Prebuilt npm binaries currently target macOS `x64/arm64`, Linux `x64` glibc,
and Windows `x64`.

### Setup

```bash
git clone https://github.com/JeremyDev87/legolas.git
cd legolas
cargo test --workspace
cargo run -p legolas-cli -- help
```

## Project Shape

Legolas is now a Rust CLI workspace, with a thin npm launcher and packaging
helpers kept in JavaScript.

```text
legolas/
├── crates/        # Rust CLI and core analysis crates
├── bin/           # npm launcher
├── test/          # launcher platform contract tests
├── scripts/       # packaging and vendor helpers
├── tests/         # parity fixtures and oracles
```

## How To Contribute

1. Create a branch for your change.
2. Keep changes focused and easy to review.
3. Add or update tests when behavior changes.
4. Update documentation when commands or output change.
5. Open a pull request using the repository template.

## Quality Bar

Before opening a pull request:

```bash
npm test
node --test test/launcher-platform.test.js
npm run smoke
cargo build --release -p legolas-cli
node ./scripts/stage-local-vendor-binary.mjs
node ./scripts/verify-vendor-layout.mjs
npm run pack:check
```

## Coding Guidelines

- Prefer small, composable functions.
- Keep runtime dependencies to zero unless there is a strong reason.
- Preserve human-readable CLI output.
- Favor deterministic analysis over clever heuristics that are hard to trust.
- Add tests for regressions, especially around parsing and reporting edge cases.

## Commit Style

Conventional Commits are recommended:

```text
feat: add webpack stats parser
fix: ignore import text inside vue template blocks
docs: clarify optimize command output
```

## Pull Requests

Include:

- what changed
- why it changed
- how it was tested
- any follow-up work or limitations

## Releases

Legolas release automation uses `package.json` as the version source of truth.

Typical release flow:

1. Start the `Manual Release Bump` workflow from GitHub Actions with the target
   tag, such as `v1.0.0`. Use `dry_run` first when you only want to validate
   the bump.
2. The workflow updates `package.json`, `crates/legolas-cli/Cargo.toml`, and
   `Cargo.lock`, then opens or reuses a draft bump PR against `master`.
3. Before merging the bump PR, confirm the PR branch has passed:
   - `legolas-ci`
   - `Release Candidate Core Verification` for the exact candidate commit SHA
4. If validating the same state locally, run:

```bash
npm test
node --test test/launcher-platform.test.js
npm run smoke
cargo build --release -p legolas-cli
node ./scripts/stage-local-vendor-binary.mjs
node ./scripts/verify-vendor-layout.mjs
node ./bin/legolas.js --version
npm run pack:check
```

5. Merge the version bump PR to `master` only after those gates pass.
6. After the merge, confirm `legolas-ci` and `Release Candidate Core
   Verification` both pass for the merged `master` commit SHA. If the candidate
   verification did not start automatically, dispatch it with that exact SHA.
7. Create and push the matching git tag, such as `v1.0.0`, from the verified
   `master` commit.
8. The `legolas-release` workflow validates the exact tag, builds native
   binaries, publishes the npm package, uploads GitHub release assets, and then
   publishes the GitHub release.
9. If a tag release must be retried, use the `legolas-release` workflow
   dispatch with the existing tag rather than creating another tag.

Thank you for helping make Legolas more useful and more trustworthy.
