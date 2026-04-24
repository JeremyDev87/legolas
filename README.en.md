# Legolas

<p align="center">
  <a href="./README.md">한국어</a> |
  <strong>English</strong> |
  <a href="./README.zh-CN.md">中文</a> |
  <a href="./README.es.md">Español</a> |
  <a href="./README.ja.md">日本語</a>
</p>

**Slim bundles with precision.**

Legolas is a Rust-powered CLI, distributed through npm with native binaries, for finding bundle-weight problems in modern web projects. It combines source-import analysis, lockfile inspection, optional bundle-artifact evidence, budget gates, and machine-readable output so optimization work can move from local triage to CI.

## What It Checks

- Framework and project shape for Next.js, Vite, Webpack, Rollup, Astro, Nuxt, React, Vue, and Svelte projects
- Static and dynamic imports in JavaScript, TypeScript, JSX, TSX, Vue, and Svelte files
- Heavy client dependencies such as charting, editor, icon, SDK, animation, map, monitoring, and UI packages
- Duplicate package versions from npm, pnpm, and Yarn lockfiles
- Tree-shaking risks, including broad icon imports, root utility imports, and repeated locale subpath imports
- Lazy-loading opportunities on route-like, dashboard, modal, editor, map, and chart surfaces
- Server/client boundary warnings for patterns such as browser surfaces importing Node-only modules
- Bundle artifacts when present, including Webpack `stats.json` and esbuild/Rollup `meta.json` files in known locations

Legolas estimates savings directionally. Treat the numbers as prioritization signals, then confirm production impact with your own bundle analyzer and performance telemetry.

## Install and Run

Run without adding a dependency:

```bash
npx @jeremyfellaz/legolas scan .
npx @jeremyfellaz/legolas visualize .
npx @jeremyfellaz/legolas optimize .
```

Or install it in a project:

```bash
npm install -D @jeremyfellaz/legolas
npx legolas scan .
```

The npm package requires Node.js `>=18.17` and ships prebuilt Rust binaries for macOS `arm64/x64`, Linux `x64` with glibc, and Windows `x64`.

## Commands

| Command | Purpose | Common options |
| --- | --- | --- |
| `scan` | Full analysis report with dependency, lockfile, import, artifact, and boundary findings | `[path]`, `--config`, `--json`, `--sarif`, `--write-baseline`, `--baseline`, `--regression-only` |
| `visualize` | Text bars for estimated dependency weight and duplicate package pressure | `[path]`, `--config`, `--limit` |
| `optimize` | Ranked action list with difficulty, confidence, target files, and suggested fixes | `[path]`, `--config`, `--top`, `--json`, `--baseline`, `--regression-only` |
| `budget` | Evaluates bundle-health budget rules | `[path]`, `--config`, `--json`, `--baseline`, `--regression-only` |
| `ci` | CI-oriented budget gate that exits with code `1` on failures | `[path]`, `--config`, `--json`, `--sarif`, `--baseline`, `--regression-only` |

Use `legolas help` for the exact CLI contract.

```bash
npx @jeremyfellaz/legolas help
```

## Common Workflows

Scan an app:

```bash
npx @jeremyfellaz/legolas scan ./apps/storefront
```

Get JSON for automation:

```bash
npx @jeremyfellaz/legolas scan ./apps/storefront --json
```

Upload SARIF from a scan-capable workflow:

```bash
npx @jeremyfellaz/legolas scan ./apps/storefront --sarif
```

Create and compare a baseline:

```bash
npx @jeremyfellaz/legolas scan ./apps/storefront --write-baseline ./legolas-baseline.json --json
npx @jeremyfellaz/legolas scan ./apps/storefront --baseline ./legolas-baseline.json --regression-only --json
```

Fail CI on budget regressions:

```bash
npx @jeremyfellaz/legolas ci ./apps/storefront --baseline ./legolas-baseline.json --regression-only --sarif
```

## Configuration

Legolas automatically discovers `legolas.config.json` from the project root. You can also pass a file explicitly with `--config`.

```json
{
  "scan": {
    "path": "src"
  },
  "visualize": {
    "limit": 12
  },
  "optimize": {
    "top": 7
  },
  "budget": {
    "rules": {
      "potentialKbSaved": {
        "warnAt": 40,
        "failAt": 80
      },
      "duplicatePackageCount": {
        "warnAt": 2,
        "failAt": 4
      },
      "dynamicImportCount": {
        "warnAt": 1,
        "failAt": 0
      }
    }
  }
}
```

`potentialKbSaved` and `duplicatePackageCount` are maximum-style rules: higher actual values are worse. `dynamicImportCount` is a minimum-style rule: too few dynamic imports can warn or fail.

## Output Formats

- `scan --json` and `optimize --json` emit `legolas.analysis.v1`, documented by [docs/schema/analysis.v1.schema.json](./docs/schema/analysis.v1.schema.json).
- `budget --json` emits `legolas.budget.v1`, documented by [docs/schema/budget.v1.schema.json](./docs/schema/budget.v1.schema.json).
- `ci --json` emits `legolas.ci.v1`, documented by [docs/schema/ci.v1.schema.json](./docs/schema/ci.v1.schema.json).
- `scan --sarif` and `ci --sarif` emit SARIF `2.1.0`, documented by [docs/schema/sarif.v1.json](./docs/schema/sarif.v1.json).

`--json` and `--sarif` are mutually exclusive. `ci` returns a non-zero exit code when budget rules fail.

## Example Output

`scan` summarizes the project, impact estimate, evidence, and finding groups:

```text
Legolas scan for basic-parity-app
Project root: <PROJECT_ROOT>
Mode: heuristic
Frameworks: Vite, React
Package manager: pnpm
Scanned 1 source files and 4 imported packages

Potential payload reduction: ~366 KB
Estimated LCP improvement: ~769 ms
High impact: the project has clear opportunities to reduce initial payload size.

Heaviest known dependencies:
- chart.js (160 KB) [high confidence]: Charting code is often only needed on a subset of screens. imported in 1 file(s).
  evidence: src/Dashboard.tsx | specifier: chart.js | static import; Charting code is often only needed on a subset of screens.
```

`optimize` turns findings into ranked actions:

```text
Legolas optimize for basic-parity-app

1. Review chart.js upfront bundle weight [hard | high confidence | ~160 KB]
   recommended fix: lazy-load - Register only the chart primitives you use and lazy load dashboard surfaces.
   targets: src/Dashboard.tsx
   evidence: src/Dashboard.tsx | specifier: chart.js | static import; Charting code is often only needed on a subset of screens.
```

`budget` reports pass, warn, or fail for each rule:

```text
Legolas budget for basic-parity-app

Overall status: Fail

Rule results:
- potentialKbSaved: Fail (actual: 366, warnAt: 40, failAt: 80)
- duplicatePackageCount: Pass (actual: 1, warnAt: 2, failAt: 4)
- dynamicImportCount: Fail (actual: 0, warnAt: 1, failAt: 0)
```

## Development

```bash
cargo run -p legolas-cli -- help
cargo test --workspace
```

Contributor workflows use `cargo run -p legolas-cli -- ...` as the source of truth. The npm package wraps the compiled Rust binary from `vendor/<triple>/legolas[.exe]`. When release packaging has staged those vendor binaries, validate the package layout with `npm run pack:check`.

## Open Source

- License: [MIT](./LICENSE)
- Contributing guide: [CONTRIBUTING.md](./CONTRIBUTING.md)
- Code of Conduct: [CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md)
- Security policy: [SECURITY.md](./SECURITY.md)
- Sponsor: [GitHub Sponsors](https://github.com/sponsors/JeremyDev87)
