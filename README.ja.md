# Legolas

<p align="center">
  <a href="./README.md">한국어</a> |
  <a href="./README.en.md">English</a> |
  <a href="./README.zh-CN.md">中文</a> |
  <a href="./README.es.md">Español</a> |
  <strong>日本語</strong>
</p>

**精密にバンドルを軽くします。**

Legolas は、npm で配布される Rust 製 CLI です。モダンな Web プロジェクトのバンドル重量問題を見つけるために、ソースのインポート解析、ロックファイル検査、必要に応じたバンドル成果物の証拠、予算ゲート、機械処理しやすい出力をまとめて提供します。

## 何を検査するか

- Next.js、Vite、Webpack、Rollup、Astro、Nuxt、React、Vue、Svelte プロジェクトのフレームワークと構成
- JavaScript、TypeScript、JSX、TSX、Vue、Svelte ファイルの静的インポートと動的インポート
- チャート、エディタ、アイコン、SDK、アニメーション、地図、監視、UI 系の重いクライアント依存
- npm、pnpm、Yarn のロックファイルにある重複パッケージバージョン
- 広すぎるアイコンインポート、ルートユーティリティインポート、繰り返しのロケールサブパスインポートなどのツリーシェイキングリスク
- ルート、ダッシュボード、モーダル、エディタ、地図、チャート領域の遅延読み込み候補
- ブラウザ側の領域が Node 専用モジュールを取り込むようなサーバー/クライアント境界の警告
- 既知の場所にある Webpack `stats.json`、esbuild/Rollup `meta.json` などのバンドル成果物

Legolas の削減見積もりは優先順位付けのための方向性シグナルです。実際の配信影響は、プロジェクト側のバンドル解析ツールとパフォーマンス計測で確認してください。

## インストールと実行

依存関係を追加せずに実行できます。

```bash
npx @jeremyfellaz/legolas scan .
npx @jeremyfellaz/legolas visualize .
npx @jeremyfellaz/legolas optimize .
```

プロジェクトの開発依存として追加することもできます。

```bash
npm install -D @jeremyfellaz/legolas
npx legolas scan .
```

npm パッケージは Node.js `>=18.17` を必要とします。同梱の Rust バイナリは macOS `arm64/x64`、glibc 版 Linux `x64`、Windows `x64` をサポートします。

## コマンド

| コマンド | 用途 | 主なオプション |
| --- | --- | --- |
| `scan` | 依存、ロックファイル、インポート、成果物、境界警告を含む完全な解析レポート | `[path]`, `--config`, `--json`, `--sarif`, `--write-baseline`, `--baseline`, `--regression-only` |
| `visualize` | 推定依存重量と重複パッケージ圧力をテキストバーで表示 | `[path]`, `--config`, `--limit` |
| `optimize` | 難易度、信頼度、対象ファイル、推奨修正を含む優先順位付きアクション一覧 | `[path]`, `--config`, `--top`, `--json`, `--baseline`, `--regression-only` |
| `budget` | バンドル健全性の予算ルールを評価 | `[path]`, `--config`, `--json`, `--baseline`, `--regression-only` |
| `ci` | 予算ルール失敗時に終了コード `1` を返す CI 向けゲート | `[path]`, `--config`, `--json`, `--sarif`, `--baseline`, `--regression-only` |

正確な CLI 契約は `legolas help` で確認できます。

```bash
npx @jeremyfellaz/legolas help
```

## よく使う流れ

アプリをスキャンします。

```bash
npx @jeremyfellaz/legolas scan ./apps/storefront
```

自動化向けに JSON を出力します。

```bash
npx @jeremyfellaz/legolas scan ./apps/storefront --json
```

スキャン結果を SARIF として出力します。

```bash
npx @jeremyfellaz/legolas scan ./apps/storefront --sarif
```

基準線を作成して比較します。

```bash
npx @jeremyfellaz/legolas scan ./apps/storefront --write-baseline ./legolas-baseline.json --json
npx @jeremyfellaz/legolas scan ./apps/storefront --baseline ./legolas-baseline.json --regression-only --json
```

予算回帰がある場合に CI を失敗させます。

```bash
npx @jeremyfellaz/legolas ci ./apps/storefront --baseline ./legolas-baseline.json --regression-only --sarif
```

## 設定

Legolas はプロジェクトルートの `legolas.config.json` を自動検出します。`--config` で設定ファイルを明示することもできます。

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

ソーススキャンはプロジェクトの `.gitignore` とルートの `.legolasignore` も反映します。`scan.ignorePatterns` は解決済みプロジェクトルート基準の POSIX スタイルのパスパターンで、`.gitignore` と同じように `!` の例外パターンを使えます。

`potentialKbSaved` と `duplicatePackageCount` は最大値ルールです。実際の値が高いほど悪い状態です。`dynamicImportCount` は最小値ルールです。動的インポートが少なすぎると警告または失敗になります。

## 出力形式

- `scan --json` と `optimize --json` は [docs/schema/analysis.v1.schema.json](./docs/schema/analysis.v1.schema.json) で文書化された `legolas.analysis.v1` を出力します。
- `budget --json` は [docs/schema/budget.v1.schema.json](./docs/schema/budget.v1.schema.json) で文書化された `legolas.budget.v1` を出力します。
- `ci --json` は [docs/schema/ci.v1.schema.json](./docs/schema/ci.v1.schema.json) で文書化された `legolas.ci.v1` を出力します。
- `scan --sarif` と `ci --sarif` は [docs/schema/sarif.v1.json](./docs/schema/sarif.v1.json) で文書化された SARIF `2.1.0` を出力します。

`--json` と `--sarif` は同時に使えません。`ci` は予算ルールが失敗すると 0 以外の終了コードを返します。

## 出力例

以下のブロックは CLI の実出力例なので英語のまま掲載しています。

`scan` はプロジェクト概要、影響見積もり、証拠、発見グループを表示します。

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

`optimize` は発見内容を優先順位付きアクションに変換します。

```text
Legolas optimize for basic-parity-app

1. Review chart.js upfront bundle weight [hard | high confidence | ~160 KB]
   recommended fix: lazy-load - Register only the chart primitives you use and lazy load dashboard surfaces.
   targets: src/Dashboard.tsx
   evidence: src/Dashboard.tsx | specifier: chart.js | static import; Charting code is often only needed on a subset of screens.
```

`budget` は各ルールの合格、警告、失敗を表示します。

```text
Legolas budget for basic-parity-app

Overall status: Fail

Rule results:
- potentialKbSaved: Fail (actual: 366, warnAt: 40, failAt: 80)
- duplicatePackageCount: Pass (actual: 1, warnAt: 2, failAt: 4)
- dynamicImportCount: Fail (actual: 0, warnAt: 1, failAt: 0)
```

## 開発

```bash
cargo run -p legolas-cli -- help
cargo test --workspace
```

コントリビューター向けの作業では `cargo run -p legolas-cli -- ...` を基準にします。npm パッケージは `vendor/<triple>/legolas[.exe]` にあるコンパイル済み Rust バイナリを実行します。リリース用のパッケージングで vendor バイナリを配置した後は、`npm run pack:check` でパッケージ構成を検証します。

## オープンソース

- ライセンス: [MIT](./LICENSE)
- コントリビューションガイド: [CONTRIBUTING.md](./CONTRIBUTING.md)
- 行動規範: [CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md)
- セキュリティポリシー: [SECURITY.md](./SECURITY.md)
- スポンサー: [GitHub Sponsors](https://github.com/sponsors/JeremyDev87)
