import _ from "lodash";
import { Chart } from "chart.js";
import { FaUser } from "react-icons";

export function AdminDashboard() {
  return [_.shuffle([1, 2, 3]), Chart, FaUser].length;
}
