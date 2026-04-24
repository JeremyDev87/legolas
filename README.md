# Legolas

<p align="center">
  <strong>한국어</strong> |
  <a href="./README.en.md">English</a> |
  <a href="./README.zh-CN.md">中文</a> |
  <a href="./README.es.md">Español</a> |
  <a href="./README.ja.md">日本語</a>
</p>

**정밀하게 번들을 줄입니다.**

Legolas는 npm으로 배포되는 Rust 기반 CLI입니다. 최신 웹 프로젝트에서 번들 무게 문제를 찾기 위해 소스 가져오기 분석, 잠금 파일 검사, 선택적 번들 산출물 증거, 예산 게이트, 기계가 읽을 수 있는 출력을 함께 제공합니다.

## 무엇을 검사하나요

- Next.js, Vite, Webpack, Rollup, Astro, Nuxt, React, Vue, Svelte 프로젝트의 프레임워크와 프로젝트 형태
- JavaScript, TypeScript, JSX, TSX, Vue, Svelte 파일의 정적 가져오기와 동적 가져오기
- 차트, 에디터, 아이콘, SDK, 애니메이션, 지도, 모니터링, UI 계열의 무거운 클라이언트 의존성
- npm, pnpm, Yarn 잠금 파일에서 발견되는 중복 패키지 버전
- 넓은 아이콘 가져오기, 루트 유틸리티 가져오기, 반복되는 지역화 하위 경로 가져오기 같은 트리 셰이킹 위험
- 라우트, 대시보드, 모달, 에디터, 지도, 차트 영역의 지연 로딩 후보
- 브라우저 영역이 Node 전용 모듈을 가져오는 경우 같은 서버/클라이언트 경계 경고
- Webpack `stats.json`, esbuild/Rollup `meta.json` 등 알려진 위치의 번들 산출물

Legolas의 절감 추정치는 우선순위를 정하기 위한 방향성 신호입니다. 실제 배포 영향은 프로젝트의 번들 분석기와 성능 텔레메트리로 확인하세요.

## 설치와 실행

의존성을 추가하지 않고 바로 실행할 수 있습니다.

```bash
npx @jeremyfellaz/legolas scan .
npx @jeremyfellaz/legolas visualize .
npx @jeremyfellaz/legolas optimize .
```

프로젝트 개발 의존성으로 설치할 수도 있습니다.

```bash
npm install -D @jeremyfellaz/legolas
npx legolas scan .
```

npm 패키지는 Node.js `>=18.17`을 요구합니다. 포함된 Rust 바이너리는 macOS `arm64/x64`, glibc 기반 Linux `x64`, Windows `x64`를 지원합니다.

## 명령어

| 명령어 | 용도 | 자주 쓰는 옵션 |
| --- | --- | --- |
| `scan` | 의존성, 잠금 파일, 가져오기, 산출물, 경계 경고를 포함한 전체 분석 보고서 | `[path]`, `--config`, `--json`, `--sarif`, `--write-baseline`, `--baseline`, `--regression-only` |
| `visualize` | 추정 의존성 무게와 중복 패키지 압력을 텍스트 막대로 표시 | `[path]`, `--config`, `--limit` |
| `optimize` | 난이도, 신뢰도, 대상 파일, 권장 수정안을 포함한 우선순위 작업 목록 | `[path]`, `--config`, `--top`, `--json`, `--baseline`, `--regression-only` |
| `budget` | 번들 상태 예산 규칙 평가 | `[path]`, `--config`, `--json`, `--baseline`, `--regression-only` |
| `ci` | 예산 실패 시 종료 코드 `1`을 반환하는 CI용 게이트 | `[path]`, `--config`, `--json`, `--sarif`, `--baseline`, `--regression-only` |

정확한 CLI 계약은 `legolas help`로 확인할 수 있습니다.

```bash
npx @jeremyfellaz/legolas help
```

## 자주 쓰는 흐름

앱을 스캔합니다.

```bash
npx @jeremyfellaz/legolas scan ./apps/storefront
```

자동화에서 사용할 JSON을 출력합니다.

```bash
npx @jeremyfellaz/legolas scan ./apps/storefront --json
```

스캔 결과를 SARIF로 출력합니다.

```bash
npx @jeremyfellaz/legolas scan ./apps/storefront --sarif
```

기준선을 만들고 비교합니다.

```bash
npx @jeremyfellaz/legolas scan ./apps/storefront --write-baseline ./legolas-baseline.json --json
npx @jeremyfellaz/legolas scan ./apps/storefront --baseline ./legolas-baseline.json --regression-only --json
```

예산 회귀가 있으면 CI를 실패시킵니다.

```bash
npx @jeremyfellaz/legolas ci ./apps/storefront --baseline ./legolas-baseline.json --regression-only --sarif
```

## 설정

Legolas는 프로젝트 루트의 `legolas.config.json`을 자동으로 찾습니다. `--config`로 설정 파일을 직접 지정할 수도 있습니다.

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

`potentialKbSaved`와 `duplicatePackageCount`는 최댓값 규칙입니다. 실제 값이 커질수록 나쁩니다. `dynamicImportCount`는 최솟값 규칙입니다. 동적 가져오기가 너무 적으면 경고나 실패가 발생할 수 있습니다.

## 출력 형식

- `scan --json`과 `optimize --json`은 [docs/schema/analysis.v1.schema.json](./docs/schema/analysis.v1.schema.json)에 문서화된 `legolas.analysis.v1`을 출력합니다.
- `budget --json`은 [docs/schema/budget.v1.schema.json](./docs/schema/budget.v1.schema.json)에 문서화된 `legolas.budget.v1`을 출력합니다.
- `ci --json`은 [docs/schema/ci.v1.schema.json](./docs/schema/ci.v1.schema.json)에 문서화된 `legolas.ci.v1`을 출력합니다.
- `scan --sarif`와 `ci --sarif`는 [docs/schema/sarif.v1.json](./docs/schema/sarif.v1.json)에 문서화된 SARIF `2.1.0`을 출력합니다.

`--json`과 `--sarif`는 함께 사용할 수 없습니다. `ci`는 예산 규칙 실패 시 0이 아닌 종료 코드를 반환합니다.

## 결과 예시

아래 블록은 실제 CLI 출력 예시이므로 영어 원문을 유지합니다.

`scan`은 프로젝트 요약, 영향 추정치, 증거, 발견 항목 그룹을 보여줍니다.

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

`optimize`는 발견 항목을 우선순위 작업으로 바꿉니다.

```text
Legolas optimize for basic-parity-app

1. Review chart.js upfront bundle weight [hard | high confidence | ~160 KB]
   recommended fix: lazy-load - Register only the chart primitives you use and lazy load dashboard surfaces.
   targets: src/Dashboard.tsx
   evidence: src/Dashboard.tsx | specifier: chart.js | static import; Charting code is often only needed on a subset of screens.
```

`budget`은 각 규칙의 통과, 경고, 실패 상태를 보여줍니다.

```text
Legolas budget for basic-parity-app

Overall status: Fail

Rule results:
- potentialKbSaved: Fail (actual: 366, warnAt: 40, failAt: 80)
- duplicatePackageCount: Pass (actual: 1, warnAt: 2, failAt: 4)
- dynamicImportCount: Fail (actual: 0, warnAt: 1, failAt: 0)
```

## 개발

```bash
cargo run -p legolas-cli -- help
cargo test --workspace
```

기여자 워크플로우는 `cargo run -p legolas-cli -- ...`를 기준으로 합니다. npm 패키지는 `vendor/<triple>/legolas[.exe]`에 포함된 컴파일된 Rust 바이너리를 실행합니다. 릴리스 패키징 과정에서 vendor 바이너리를 준비한 뒤에는 `npm run pack:check`로 패키지 구성을 검증합니다.

## 오픈소스

- 라이선스: [MIT](./LICENSE)
- 기여 안내: [CONTRIBUTING.md](./CONTRIBUTING.md)
- 행동 강령: [CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md)
- 보안 정책: [SECURITY.md](./SECURITY.md)
- 후원: [GitHub Sponsors](https://github.com/sponsors/JeremyDev87)
