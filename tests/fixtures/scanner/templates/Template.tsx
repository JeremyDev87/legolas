const fakeLoader = `import("react-icons") require("lodash")`;
const nested = `export { Chart } from "chart.js/helpers"`;

export const Template = `${fakeLoader} ${nested}`;
