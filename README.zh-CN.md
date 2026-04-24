# Legolas

<p align="center">
  <a href="./README.md">한국어</a> |
  <a href="./README.en.md">English</a> |
  <strong>中文</strong> |
  <a href="./README.es.md">Español</a> |
  <a href="./README.ja.md">日本語</a>
</p>

**精准瘦身前端包体积。**

Legolas 是一个通过 npm 分发、由 Rust 驱动的 CLI。它面向现代 Web 项目的包体积问题，结合源码导入分析、锁文件检查、可选的构建产物证据、预算门禁以及机器可读输出，帮助把本地排查结果接入持续集成流程。

## 检查内容

- Next.js、Vite、Webpack、Rollup、Astro、Nuxt、React、Vue、Svelte 项目的框架和项目形态
- JavaScript、TypeScript、JSX、TSX、Vue、Svelte 文件中的静态导入和动态导入
- 图表、编辑器、图标、SDK、动画、地图、监控、界面组件等较重的客户端依赖
- npm、pnpm、Yarn 锁文件中的重复包版本
- 影响摇树优化的风险，例如宽泛图标导入、根工具库导入、重复本地化子路径导入
- 路由、仪表盘、弹窗、编辑器、地图、图表区域的延迟加载候选项
- 服务端/客户端边界警告，例如浏览器侧代码导入 Node 专用模块
- 已知位置中的构建产物，例如 Webpack `stats.json` 和 esbuild/Rollup `meta.json`

Legolas 的节省估算用于确定优化优先级。真实上线影响仍应通过项目自己的包分析器和性能遥测来确认。

## 安装与运行

无需添加依赖即可运行：

```bash
npx @jeremyfellaz/legolas scan .
npx @jeremyfellaz/legolas visualize .
npx @jeremyfellaz/legolas optimize .
```

也可以安装到项目中：

```bash
npm install -D @jeremyfellaz/legolas
npx legolas scan .
```

npm 包要求 Node.js `>=18.17`，并内置面向 macOS `arm64/x64`、glibc Linux `x64`、Windows `x64` 的预构建 Rust 二进制。

## 命令

| 命令 | 用途 | 常用选项 |
| --- | --- | --- |
| `scan` | 输出包含依赖、锁文件、导入、构建产物、边界警告的完整分析报告 | `[path]`, `--config`, `--json`, `--sarif`, `--write-baseline`, `--baseline`, `--regression-only` |
| `visualize` | 用文本条展示估算依赖重量和重复包压力 | `[path]`, `--config`, `--limit` |
| `optimize` | 输出按优先级排序的操作列表，包含难度、可信度、目标文件和建议修复方式 | `[path]`, `--config`, `--top`, `--json`, `--baseline`, `--regression-only` |
| `budget` | 评估包体积健康预算规则 | `[path]`, `--config`, `--json`, `--baseline`, `--regression-only` |
| `ci` | 面向持续集成的门禁，预算规则失败时返回退出码 `1` | `[path]`, `--config`, `--json`, `--sarif`, `--baseline`, `--regression-only` |

使用 `legolas help` 查看准确的 CLI 契约。

```bash
npx @jeremyfellaz/legolas help
```

## 常用流程

扫描一个应用：

```bash
npx @jeremyfellaz/legolas scan ./apps/storefront
```

输出自动化可用的 JSON：

```bash
npx @jeremyfellaz/legolas scan ./apps/storefront --json
```

以 SARIF 输出扫描结果：

```bash
npx @jeremyfellaz/legolas scan ./apps/storefront --sarif
```

创建并比较基线：

```bash
npx @jeremyfellaz/legolas scan ./apps/storefront --write-baseline ./legolas-baseline.json --json
npx @jeremyfellaz/legolas scan ./apps/storefront --baseline ./legolas-baseline.json --regression-only --json
```

当预算回归出现时让持续集成失败：

```bash
npx @jeremyfellaz/legolas ci ./apps/storefront --baseline ./legolas-baseline.json --regression-only --sarif
```

## 配置

Legolas 会从项目根目录自动发现 `legolas.config.json`。也可以通过 `--config` 显式指定配置文件。

```json
{
  "scan": {
    "path": "src",
    "ignorePatterns": ["generated/**", "!generated/keep.ts"]
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

源代码扫描也会遵循项目的 `.gitignore` 和根目录 `.legolasignore`。`scan.ignorePatterns` 使用相对于解析后项目根目录的 POSIX 风格路径模式，并像 `.gitignore` 一样支持 `!` 例外模式。

`potentialKbSaved` 和 `duplicatePackageCount` 是最大值规则，实际值越高越差。`dynamicImportCount` 是最小值规则，动态导入过少时可能触发警告或失败。

## 输出格式

- `scan --json` 和 `optimize --json` 输出 `legolas.analysis.v1`，结构见 [docs/schema/analysis.v1.schema.json](./docs/schema/analysis.v1.schema.json)。
- `budget --json` 输出 `legolas.budget.v1`，结构见 [docs/schema/budget.v1.schema.json](./docs/schema/budget.v1.schema.json)。
- `ci --json` 输出 `legolas.ci.v1`，结构见 [docs/schema/ci.v1.schema.json](./docs/schema/ci.v1.schema.json)。
- `scan --sarif` 和 `ci --sarif` 输出 SARIF `2.1.0`，结构见 [docs/schema/sarif.v1.json](./docs/schema/sarif.v1.json)。

`--json` 和 `--sarif` 不能同时使用。`ci` 在预算规则失败时返回非零退出码。

## 结果示例

以下代码块是 CLI 的真实输出示例，因此保留英文原文。

`scan` 会汇总项目、影响估算、证据和发现项分组：

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

`optimize` 会把发现项转换为按优先级排序的操作：

```text
Legolas optimize for basic-parity-app

1. Review chart.js upfront bundle weight [hard | high confidence | ~160 KB]
   recommended fix: lazy-load - Register only the chart primitives you use and lazy load dashboard surfaces.
   targets: src/Dashboard.tsx
   evidence: src/Dashboard.tsx | specifier: chart.js | static import; Charting code is often only needed on a subset of screens.
```

`budget` 会报告每条规则的通过、警告或失败状态：

```text
Legolas budget for basic-parity-app

Overall status: Fail

Rule results:
- potentialKbSaved: Fail (actual: 366, warnAt: 40, failAt: 80)
- duplicatePackageCount: Pass (actual: 1, warnAt: 2, failAt: 4)
- dynamicImportCount: Fail (actual: 0, warnAt: 1, failAt: 0)
```

## 开发

```bash
cargo run -p legolas-cli -- help
cargo test --workspace
```

贡献者工作流以 `cargo run -p legolas-cli -- ...` 为准。npm 包会执行 `vendor/<triple>/legolas[.exe]` 中的已编译 Rust 二进制。发布打包流程准备好 vendor 二进制之后，再使用 `npm run pack:check` 验证包结构。

## 开源

- 许可证：[MIT](./LICENSE)
- 贡献指南：[CONTRIBUTING.md](./CONTRIBUTING.md)
- 行为准则：[CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md)
- 安全政策：[SECURITY.md](./SECURITY.md)
- 赞助：[GitHub Sponsors](https://github.com/sponsors/JeremyDev87)
