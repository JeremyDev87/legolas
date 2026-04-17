# Legolas

<p align="center">
  <a href="./README.md">한국어</a> |
  <a href="./README.en.md">English</a> |
  <a href="./README.zh-CN.md">中文</a> |
  <strong>Español</strong> |
  <a href="./README.ja.md">日本語</a>
</p>

Slim bundles with precision.

Legolas es una CLI sin dependencias de runtime que inspecciona proyectos web modernos para detectar peso de bundle, paquetes duplicados, fallos de tree-shaking y oportunidades de lazy loading.

## Por Qué Legolas

Las aplicaciones web modernas rara vez se vuelven lentas por un solo gran error. Normalmente se degradan cuando se acumulan pequeñas fuentes de peso:

- dependencias cliente demasiado grandes
- versiones duplicadas del mismo paquete
- imports estáticos que deberían cargarse más tarde
- imports de iconos y utilidades que debilitan el tree shaking

Legolas analiza esas señales y las convierte en un informe de optimización fácil de leer y accionar.

## Comandos

```bash
npx legolas scan
npx legolas visualize
npx legolas optimize
```

También puedes analizar una ruta de proyecto específica:

```bash
node ./bin/legolas.js scan ./apps/storefront
node ./bin/legolas.js visualize . --limit 12
node ./bin/legolas.js optimize . --top 7
```

## Qué Hace El MVP Actual

- detecta frameworks como Next.js, Vite, Webpack, Astro, Nuxt, React, Vue y Svelte
- identifica dependencias frontend pesadas usando una base de conocimiento curada
- analiza `package-lock.json`, `pnpm-lock.yaml` y `yarn.lock` para encontrar versiones duplicadas
- escanea archivos fuente para detectar imports estáticos, dinámicos y anti-patrones de tree-shaking
- recomienda candidatos para lazy loading en superficies como chart, editor, map, dashboard, modal y routes
- estima de forma orientativa el ahorro de payload y la mejora de LCP

## Ejemplo

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

## Desarrollo

```bash
npm test
node ./bin/legolas.js help
```

## Código Abierto

- License: [MIT](./LICENSE)
- Contributing guide: [CONTRIBUTING.md](./CONTRIBUTING.md)
- Code of Conduct: [CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md)
- Security policy: [SECURITY.md](./SECURITY.md)
- Sponsor: [GitHub Sponsors](https://github.com/sponsors/JeremyDev87)

## Notas

- La versión actual es principalmente heurística. Si existen artefactos de bundle como `stats.json` o `meta.json`, Legolas los detecta, pero el análisis completo basado en artifacts sigue siendo el siguiente paso natural.
- La CLI evita a propósito dependencias externas de runtime para que cualquier colaborador pueda clonar y ejecutarla de inmediato.
