import _ from "lodash";
import { FaUser } from "react-icons";
import { Button } from "@mui/material/Button";
const utc = require("dayjs/plugin/utc");

export { Chart } from "chart.js";

export function Dashboard() {
  return <section>{_.shuffle([FaUser, Button, utc, Chart]).length}</section>;
}
