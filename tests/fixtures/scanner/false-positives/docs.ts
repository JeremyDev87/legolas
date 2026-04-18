const prose = "import lodash from 'lodash'";
const snippet = 'require("chart.js")';
const template = `import("react-icons")`;
import ignoredScopedRoot from "@scope/";
import ignoredScopedSubpath from "@scope//subpath";

/*
import maps from "mapbox-gl";
export { fake } from "react";
*/

export const docs = {
  import: "keyword property",
  require: "not a call",
  code: "export from text",
  ignoredScopedRoot,
  ignoredScopedSubpath,
};
