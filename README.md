# Legolas

<p align="center">
  <strong>한국어</strong> |
  <a href="./README.en.md">English</a> |
  <a href="./README.zh-CN.md">中文</a> |
  <a href="./README.es.md">Español</a> |
  <a href="./README.ja.md">日本語</a>
</p>

Slim bundles with precision.

> Quick Overview (EN): Legolas is a zero-dependency CLI for analyzing bundle weight, duplicate packages, tree-shaking misses, and lazy-loading opportunities in modern web projects.
>
> Quick Start:
> ```bash
> npx legolas scan
> npx legolas visualize
> npx legolas optimize
> ```

Legolas는 최신 웹 프로젝트의 번들 크기, 중복 패키지, tree-shaking 누수, lazy loading 기회를 점검하는 zero-dependency CLI입니다.

## 왜 Legolas인가

현대 웹앱은 한 번의 큰 실수 때문에만 느려지지 않습니다. 보통 아래 같은 작은 무게들이 쌓이면서 점점 느려집니다.

- 불필요하게 큰 클라이언트 의존성
- 여러 버전으로 중복 설치된 패키지
- 나중에 불러와도 되는 정적 import
- tree-shaking을 방해하는 아이콘/유틸리티 import

Legolas는 이런 신호들을 스캔해서 사람이 바로 읽고 판단할 수 있는 최적화 리포트로 바꿔줍니다.

## 명령어

```bash
npx legolas scan
npx legolas visualize
npx legolas optimize
```

특정 프로젝트 경로를 직접 넘겨서 분석할 수도 있습니다.

```bash
node ./bin/legolas.js scan ./apps/storefront
node ./bin/legolas.js visualize . --limit 12
node ./bin/legolas.js optimize . --top 7
```

## 현재 MVP가 하는 일

- Next.js, Vite, Webpack, Astro, Nuxt, React, Vue, Svelte 같은 프로젝트 프레임워크 감지
- 사전 정의된 지식 베이스를 이용해 무거운 프론트엔드 의존성 식별
- `package-lock.json`, `pnpm-lock.yaml`, `yarn.lock`에서 중복 패키지 버전 탐지
- 소스 파일에서 정적 import, 동적 import, tree-shaking 안티패턴 스캔
- chart, editor, map, dashboard, modal, route 성격의 영역에서 lazy loading 후보 추천
- 절감 가능한 payload 크기와 LCP 개선 폭을 방향성 기준으로 추정

## 예시

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

## 개발

```bash
npm test
node ./bin/legolas.js help
```

## 오픈소스

- License: [MIT](./LICENSE)
- Contributing guide: [CONTRIBUTING.md](./CONTRIBUTING.md)
- Code of Conduct: [CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md)
- Security policy: [SECURITY.md](./SECURITY.md)
- Sponsor: [GitHub Sponsors](https://github.com/sponsors/JeremyDev87)

## 참고

- 현재 릴리스는 heuristic-first 접근입니다. `stats.json`, `meta.json` 같은 번들 산출물이 있으면 존재는 감지하지만, artifact-native 정밀 분석은 다음 단계의 자연스러운 확장입니다.
- 이 CLI는 기여자가 바로 clone 해서 실행할 수 있도록 런타임 외부 의존성을 두지 않는 방향을 유지합니다.
