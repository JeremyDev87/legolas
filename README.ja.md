# Legolas

<p align="center">
  <a href="./README.md">한국어</a> |
  <a href="./README.en.md">English</a> |
  <a href="./README.zh-CN.md">中文</a> |
  <a href="./README.es.md">Español</a> |
  <strong>日本語</strong>
</p>

Slim bundles with precision.

Legolas は、npm パッケージ内に native Rust binary を同梱して配布する CLI で、モダンな Web プロジェクトのバンドルサイズ、重複パッケージ、tree-shaking の取りこぼし、lazy loading の余地を点検します。

## なぜ Legolas か

現代の Web アプリは、ひとつの大きな失敗だけで遅くなるわけではありません。多くの場合、次のような小さな重さが積み重なって遅くなります。

- 大きすぎるクライアント依存
- 複数バージョンで重複しているパッケージ
- 後から読み込めるのに静的 import されているコード
- tree-shaking を弱めるアイコンやユーティリティの import

Legolas はこうしたシグナルをスキャンし、人がすぐに判断できる最適化レポートへ変換します。

## コマンド

```bash
npx @jeremyfellaz/legolas scan
npx @jeremyfellaz/legolas visualize
npx @jeremyfellaz/legolas optimize
```

特定のプロジェクトパスを直接指定して解析することもできます。

```bash
cargo run -p legolas-cli -- scan ./apps/storefront
cargo run -p legolas-cli -- visualize . --limit 12
cargo run -p legolas-cli -- optimize . --top 7
```

## 現在の MVP でできること

- Next.js、Vite、Webpack、Astro、Nuxt、React、Vue、Svelte などのフレームワークを検出
- 内蔵の知識ベースを使って重いフロントエンド依存を特定
- `package-lock.json`、`pnpm-lock.yaml`、`yarn.lock` を解析して重複バージョンを検出
- ソースファイル内の静的 import、動的 import、tree-shaking のアンチパターンをスキャン
- chart、editor、map、dashboard、modal、route 系の領域で lazy loading 候補を提案
- 削減できそうな payload と LCP 改善幅を方向性ベースで推定

## 例

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

## 開発

```bash
cargo test --workspace
cargo run -p legolas-cli -- help
```

## オープンソース

- License: [MIT](./LICENSE)
- Contributing guide: [CONTRIBUTING.md](./CONTRIBUTING.md)
- Code of Conduct: [CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md)
- Security policy: [SECURITY.md](./SECURITY.md)
- Sponsor: [GitHub Sponsors](https://github.com/sponsors/JeremyDev87)

## 補足

- 現在のリリースは heuristic-first な方針です。`stats.json` や `meta.json` のようなバンドル成果物があれば存在は検出しますが、artifact-native の本格解析は次の自然な拡張です。
- 現在の npm 配布物の prebuilt binary は macOS `x64/arm64`、Linux `x64` glibc、Windows `x64` をサポートします。
- npm 配布物は `vendor/<triple>/legolas[.exe]` layout で platform ごとの Rust binary を含み、repository 側の contributor path は `cargo run -p legolas-cli -- ...` を基準にします。
