# Legolas

<p align="center">
  <a href="./README.md">한국어</a> |
  <a href="./README.en.md">English</a> |
  <strong>中文</strong> |
  <a href="./README.es.md">Español</a> |
  <a href="./README.ja.md">日本語</a>
</p>

Slim bundles with precision.

Legolas 是一个由 Rust 驱动、通过 npm 分发原生二进制的 CLI，用来检查现代 Web 项目的打包体积、重复依赖、tree-shaking 漏损以及 lazy loading 优化机会。

## 为什么选择 Legolas

现代 Web 应用通常不是因为一次巨大的错误才变慢，而是因为很多小的体积问题不断累积：

- 过大的前端依赖
- 多版本重复安装的包
- 本应延后加载的静态 import
- 削弱 tree-shaking 的图标和工具库 import

Legolas 会扫描这些信号，并把它们整理成便于人工阅读和决策的优化报告。

## 命令

```bash
npx legolas scan
npx legolas visualize
npx legolas optimize
```

也可以直接指定某个项目路径进行分析：

```bash
cargo run -p legolas-cli -- scan ./apps/storefront
cargo run -p legolas-cli -- visualize . --limit 12
cargo run -p legolas-cli -- optimize . --top 7
```

## 当前 MVP 的能力

- 检测 Next.js、Vite、Webpack、Astro、Nuxt、React、Vue、Svelte 等项目框架
- 基于内置知识库识别较重的前端依赖
- 解析 `package-lock.json`、`pnpm-lock.yaml`、`yarn.lock` 以发现重复包版本
- 扫描源码中的静态 import、动态 import 以及 tree-shaking 反模式
- 为 chart、editor、map、dashboard、modal、route 等区域推荐 lazy loading 候选项
- 提供可节省 payload 与 LCP 改善幅度的方向性估算

## 示例

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

## 开发

```bash
cargo test --workspace
cargo run -p legolas-cli -- help
```

## 开源信息

- License: [MIT](./LICENSE)
- Contributing guide: [CONTRIBUTING.md](./CONTRIBUTING.md)
- Code of Conduct: [CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md)
- Security policy: [SECURITY.md](./SECURITY.md)
- Sponsor: [GitHub Sponsors](https://github.com/sponsors/JeremyDev87)

## 说明

- 当前版本以启发式分析为主。如果项目中存在 `stats.json`、`meta.json` 之类的打包产物，Legolas 会识别到它们，但完整的 artifact-native 精确分析仍是下一阶段的自然扩展。
- 当前 npm 发布物提供 macOS `x64/arm64`、Linux `x64` glibc、Windows `x64` 的预构建二进制。
- npm 发布物会把各平台 Rust 二进制放在 `vendor/<triple>/legolas[.exe]` 下，而仓库贡献路径则以 `cargo run -p legolas-cli -- ...` 为准。
