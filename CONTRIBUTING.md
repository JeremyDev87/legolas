# Contributing to Legolas

Thank you for your interest in contributing to Legolas.

## Before You Start

- Read the [Code of Conduct](./CODE_OF_CONDUCT.md)
- Check existing [issues](https://github.com/JeremyDev87/legolas/issues)
- Open an issue first for larger feature proposals or design changes

## Development Setup

### Prerequisites

- Node.js 18.17 or newer

### Setup

```bash
git clone https://github.com/JeremyDev87/legolas.git
cd legolas
npm test
node ./bin/legolas.js help
```

## Project Shape

Legolas is currently a small zero-dependency Node CLI.

```text
legolas/
├── bin/           # CLI entrypoint
├── src/core/      # analysis engine
├── src/reporters/ # text output
└── test/          # node:test coverage
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
npm run smoke
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

1. Update `package.json` to the next version.
2. Verify the CLI reports the same version with `node ./bin/legolas.js --version`.
3. Run:

```bash
npm test
npm run smoke
npm run pack:check
```

4. Merge the version bump to `master`.
5. Push a matching git tag such as `v0.1.1`.
6. GitHub Actions validates the tag, publishes to npm, and then publishes the GitHub release.

Thank you for helping make Legolas more useful and more trustworthy.
