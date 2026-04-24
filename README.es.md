# Legolas

<p align="center">
  <a href="./README.md">한국어</a> |
  <a href="./README.en.md">English</a> |
  <a href="./README.zh-CN.md">中文</a> |
  <strong>Español</strong> |
  <a href="./README.ja.md">日本語</a>
</p>

**Paquetes más ligeros, con precisión.**

Legolas es una CLI impulsada por Rust y distribuida por npm con binarios nativos. Sirve para encontrar problemas de peso del paquete generado en proyectos web modernos mediante análisis de importaciones de código fuente, revisión de archivos de bloqueo, evidencia opcional de artefactos de empaquetado, reglas de presupuesto y salidas legibles por máquinas.

## Qué analiza

- Forma del proyecto y marcos de trabajo como Next.js, Vite, Webpack, Rollup, Astro, Nuxt, React, Vue y Svelte
- Importaciones estáticas y dinámicas en archivos JavaScript, TypeScript, JSX, TSX, Vue y Svelte
- Dependencias cliente pesadas de gráficos, editores, iconos, SDK, animación, mapas, monitoreo e interfaces
- Versiones duplicadas de paquetes en archivos de bloqueo de npm, pnpm y Yarn
- Riesgos para la eliminación de código no usado, como importaciones amplias de iconos, importaciones raíz de utilidades e importaciones repetidas de subrutas de localización
- Oportunidades de carga diferida en rutas, paneles, modales, editores, mapas y gráficos
- Advertencias de frontera servidor/cliente, por ejemplo superficies de navegador que importan módulos exclusivos de Node
- Artefactos de empaquetado cuando existen, incluidos `stats.json` de Webpack y `meta.json` de esbuild/Rollup en ubicaciones conocidas

Las estimaciones de ahorro de Legolas son señales de priorización. Confirma el impacto real con el analizador de paquetes y la telemetría de rendimiento de tu proyecto.

## Instalación y ejecución

Puedes ejecutarlo sin añadir una dependencia:

```bash
npx @jeremyfellaz/legolas scan .
npx @jeremyfellaz/legolas visualize .
npx @jeremyfellaz/legolas optimize .
```

También puedes instalarlo en el proyecto:

```bash
npm install -D @jeremyfellaz/legolas
npx legolas scan .
```

El paquete npm requiere Node.js `>=18.17` e incluye binarios Rust precompilados para macOS `arm64/x64`, Linux `x64` con glibc y Windows `x64`.

## Comandos

| Comando | Propósito | Opciones comunes |
| --- | --- | --- |
| `scan` | Informe completo con hallazgos de dependencias, archivos de bloqueo, importaciones, artefactos y fronteras | `[path]`, `--config`, `--json`, `--sarif`, `--write-baseline`, `--baseline`, `--regression-only` |
| `visualize` | Barras de texto para peso estimado de dependencias y presión de paquetes duplicados | `[path]`, `--config`, `--limit` |
| `optimize` | Lista priorizada de acciones con dificultad, confianza, archivos objetivo y correcciones sugeridas | `[path]`, `--config`, `--top`, `--json`, `--baseline`, `--regression-only` |
| `budget` | Evalúa reglas de presupuesto de salud del paquete generado | `[path]`, `--config`, `--json`, `--baseline`, `--regression-only` |
| `ci` | Puerta para CI que devuelve código de salida `1` cuando fallan las reglas | `[path]`, `--config`, `--json`, `--sarif`, `--baseline`, `--regression-only` |

Usa `legolas help` para ver el contrato exacto de la CLI.

```bash
npx @jeremyfellaz/legolas help
```

## Flujos comunes

Analizar una aplicación:

```bash
npx @jeremyfellaz/legolas scan ./apps/storefront
```

Obtener JSON para automatización:

```bash
npx @jeremyfellaz/legolas scan ./apps/storefront --json
```

Emitir SARIF desde un análisis:

```bash
npx @jeremyfellaz/legolas scan ./apps/storefront --sarif
```

Crear y comparar una línea base:

```bash
npx @jeremyfellaz/legolas scan ./apps/storefront --write-baseline ./legolas-baseline.json --json
npx @jeremyfellaz/legolas scan ./apps/storefront --baseline ./legolas-baseline.json --regression-only --json
```

Fallar CI cuando haya regresiones de presupuesto:

```bash
npx @jeremyfellaz/legolas ci ./apps/storefront --baseline ./legolas-baseline.json --regression-only --sarif
```

## Configuración

Legolas descubre automáticamente `legolas.config.json` desde la raíz del proyecto. También puedes pasar un archivo de forma explícita con `--config`.

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

`potentialKbSaved` y `duplicatePackageCount` son reglas de máximo: valores reales más altos son peores. `dynamicImportCount` es una regla de mínimo: muy pocas importaciones dinámicas pueden producir advertencia o fallo.

## Formatos de salida

- `scan --json` y `optimize --json` emiten `legolas.analysis.v1`, documentado en [docs/schema/analysis.v1.schema.json](./docs/schema/analysis.v1.schema.json).
- `budget --json` emite `legolas.budget.v1`, documentado en [docs/schema/budget.v1.schema.json](./docs/schema/budget.v1.schema.json).
- `ci --json` emite `legolas.ci.v1`, documentado en [docs/schema/ci.v1.schema.json](./docs/schema/ci.v1.schema.json).
- `scan --sarif` y `ci --sarif` emiten SARIF `2.1.0`, documentado en [docs/schema/sarif.v1.json](./docs/schema/sarif.v1.json).

`--json` y `--sarif` no se pueden usar juntos. `ci` devuelve un código de salida distinto de cero cuando fallan las reglas de presupuesto.

## Ejemplos de resultado

Los siguientes bloques son salidas reales de la CLI, por eso se mantienen en inglés.

`scan` resume el proyecto, la estimación de impacto, la evidencia y los grupos de hallazgos:

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

`optimize` convierte hallazgos en acciones priorizadas:

```text
Legolas optimize for basic-parity-app

1. Review chart.js upfront bundle weight [hard | high confidence | ~160 KB]
   recommended fix: lazy-load - Register only the chart primitives you use and lazy load dashboard surfaces.
   targets: src/Dashboard.tsx
   evidence: src/Dashboard.tsx | specifier: chart.js | static import; Charting code is often only needed on a subset of screens.
```

`budget` informa aprobación, advertencia o fallo para cada regla:

```text
Legolas budget for basic-parity-app

Overall status: Fail

Rule results:
- potentialKbSaved: Fail (actual: 366, warnAt: 40, failAt: 80)
- duplicatePackageCount: Pass (actual: 1, warnAt: 2, failAt: 4)
- dynamicImportCount: Fail (actual: 0, warnAt: 1, failAt: 0)
```

## Desarrollo

```bash
cargo run -p legolas-cli -- help
cargo test --workspace
```

Los flujos de contribución usan `cargo run -p legolas-cli -- ...` como referencia. El paquete npm ejecuta el binario Rust compilado desde `vendor/<triple>/legolas[.exe]`. Cuando el empaquetado de una versión ya haya preparado esos binarios de vendor, valida la disposición del paquete con `npm run pack:check`.

## Código Abierto

- Licencia: [MIT](./LICENSE)
- Guía de contribución: [CONTRIBUTING.md](./CONTRIBUTING.md)
- Código de conducta: [CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md)
- Política de seguridad: [SECURITY.md](./SECURITY.md)
- Patrocinio: [GitHub Sponsors](https://github.com/sponsors/JeremyDev87)
