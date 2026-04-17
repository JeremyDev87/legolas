import { useEffect } from "react";
import _ from "lodash";
import { FaUser } from "react-icons";
import { Chart } from "chart.js";

export function Dashboard() {
  useEffect(() => {
    console.log(_.shuffle([1, 2, 3]), Chart, FaUser);
  }, []);

  return null;
}
