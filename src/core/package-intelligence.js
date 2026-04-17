const PACKAGE_INTELLIGENCE = {
  "aws-sdk": {
    estimatedKb: 700,
    category: "sdk",
    rationale: "The v2 AWS SDK is broad and frequently lands in client bundles by accident.",
    recommendation: "Move client-side calls to modular AWS SDK v3 packages or server boundaries."
  },
  firebase: {
    estimatedKb: 180,
    category: "sdk",
    rationale: "Firebase bundles grow quickly when the compat layer or multiple services are pulled together.",
    recommendation: "Use modular Firebase imports and lazy load infrequent auth or analytics flows."
  },
  "monaco-editor": {
    estimatedKb: 320,
    category: "editor",
    rationale: "Monaco is powerful but rarely belongs in the critical path.",
    recommendation: "Load Monaco only on editor routes and defer language workers until needed."
  },
  three: {
    estimatedKb: 230,
    category: "3d",
    rationale: "Three.js is often one of the heaviest client-side libraries in a web app.",
    recommendation: "Split 3D experiences behind route boundaries or on-demand interactions."
  },
  antd: {
    estimatedKb: 210,
    category: "ui",
    rationale: "Large component suites and styling layers can inflate initial chunks.",
    recommendation: "Prefer route-based splits and avoid importing broad UI modules into shared entry points."
  },
  "chart.js": {
    estimatedKb: 160,
    category: "charts",
    rationale: "Charting code is often only needed on a subset of screens.",
    recommendation: "Register only the chart primitives you use and lazy load dashboard surfaces."
  },
  "echarts": {
    estimatedKb: 260,
    category: "charts",
    rationale: "ECharts is feature-rich but rarely lightweight.",
    recommendation: "Split chart-heavy screens and consider lighter renderers for simple charts."
  },
  "react-icons": {
    estimatedKb: 90,
    category: "icons",
    rationale: "Wide icon-pack imports can defeat tree shaking.",
    recommendation: "Import narrowly from specific icon files or migrate to a more tree-shakable icon set."
  },
  "@mui/icons-material": {
    estimatedKb: 220,
    category: "icons",
    rationale: "Icons often spread across the app and are easy to over-import.",
    recommendation: "Use direct icon imports and lazy load icon-heavy admin or settings routes."
  },
  "@mui/material": {
    estimatedKb: 120,
    category: "ui",
    rationale: "Barrel imports can keep more UI code in shared chunks than intended.",
    recommendation: "Audit entry-point imports and keep heavy UI primitives out of global layouts."
  },
  lodash: {
    estimatedKb: 72,
    category: "utility",
    rationale: "Root lodash imports are a classic source of tree-shaking misses.",
    recommendation: "Use per-method imports or switch to lodash-es when the toolchain supports it."
  },
  moment: {
    estimatedKb: 67,
    category: "date",
    rationale: "Moment brings notable weight and locale baggage.",
    recommendation: "Prefer date-fns, Day.js, or the platform Intl APIs where practical."
  },
  "framer-motion": {
    estimatedKb: 85,
    category: "animation",
    rationale: "Animation libraries can be worth the cost, but not everywhere.",
    recommendation: "Restrict motion-heavy features to lazy-loaded surfaces and trim rarely used transitions."
  },
  "highlight.js": {
    estimatedKb: 110,
    category: "rendering",
    rationale: "Syntax highlighting usually belongs in secondary reading or editing views.",
    recommendation: "Load highlight grammars on demand or swap to a smaller highlighter."
  },
  "@react-google-maps/api": {
    estimatedKb: 130,
    category: "maps",
    rationale: "Map SDKs are expensive and usually route-specific.",
    recommendation: "Keep maps behind dynamic imports and avoid rendering them inside shared shells."
  },
  "@sentry/browser": {
    estimatedKb: 90,
    category: "monitoring",
    rationale: "Instrumentation can bloat entry chunks if initialized too eagerly.",
    recommendation: "Defer optional integrations and review whether client monitoring needs the full browser SDK."
  }
};

export function getPackageIntel(name) {
  return PACKAGE_INTELLIGENCE[name] ?? null;
}
