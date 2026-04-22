import { Chart } from "chart.js";
import _ from "lodash";

export default function App() {
  return Chart ? <div>{_.chunk([1, 2, 3], 2).length}</div> : null;
}
