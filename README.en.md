# Legolas

<p align="center">
  <a href="./README.md">한국어</a> |
  <strong>English</strong> |
  <a href="./README.zh-CN.md">中文</a> |
  <a href="./README.es.md">Español</a> |
  <a href="./README.ja.md">日本語</a>
</p>

Slim bundles with precision.

Legolas is a Rust-powered CLI, shipped through npm with native binaries, that inspects modern web projects for bundle weight, duplicate packages, tree-shaking misses, and lazy-loading opportunities.

## Why Legolas

Modern web apps rarely slow down because of one big mistake. They usually get slower as small sources of bundle weight accumulate:

- oversized client-side dependencies
- duplicated package versions
- static imports that should be deferred
- icon and utility imports that weaken tree shaking

Legolas scans those signals and turns them into an optimization report that humans can read and act on quickly.

## Commands

```bash
npx @jeremyfellaz/legolas scan
npx @jeremyfellaz/legolas visualize
npx @jeremyfellaz/legolas optimize
```

You can also point Legolas at a specific project path:

```bash
cargo run -p legolas-cli -- scan ./apps/storefront
cargo run -p legolas-cli -- visualize . --limit 12
cargo run -p legolas-cli -- optimize . --top 7
```

## What The Current MVP Does

- detects frameworks such as Next.js, Vite, Webpack, Astro, Nuxt, React, Vue, and Svelte
- identifies heavyweight frontend dependencies from a curated knowledge base
- parses `package-lock.json`, `pnpm-lock.yaml`, and `yarn.lock` to spot duplicate package versions
- scans source files for static imports, dynamic imports, and tree-shaking anti-patterns
- recommends lazy-loading candidates for chart, editor, map, dashboard, modal, and route-like surfaces
- estimates directional payload savings and LCP improvement

## Example

```text
Legolas scan for storefront
Project root: /workspace/storefront
Mode: heuristic
Frameworks: Next.js, React
Package manager: pnpm
Scanned 84 source files and 53 imported packages

Potential payload reduction: ~246 KB
Estimated LCP improvement: ~517 ms
Medium impact: there are several meaningful bundle wins available.
```

## Development

```bash
cargo test --workspace
cargo run -p legolas-cli -- help
```

## Open Source

- License: [MIT](./LICENSE)
- Contributing guide: [CONTRIBUTING.md](./CONTRIBUTING.md)
- Code of Conduct: [CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md)
- Security policy: [SECURITY.md](./SECURITY.md)
- Sponsor: [GitHub Sponsors](https://github.com/sponsors/JeremyDev87)

## Notes

- The current release is heuristic-first. If bundle artifacts such as `stats.json` or `meta.json` exist, Legolas detects them, but full artifact-native analysis is still the next natural step.
- The current npm package ships prebuilt binaries for macOS `x64/arm64`, Linux `x64` glibc, and Windows `x64`.
- The npm package ships platform-specific Rust binaries under `vendor/<triple>/legolas[.exe]`, while contributor workflows use `cargo run -p legolas-cli -- ...` as the source of truth.
