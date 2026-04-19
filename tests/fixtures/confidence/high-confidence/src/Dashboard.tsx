import { Chart } from "chart.js";
import _ from "lodash";

export function Dashboard() {
  return [Chart, _.shuffle([1, 2, 3])].length;
}
