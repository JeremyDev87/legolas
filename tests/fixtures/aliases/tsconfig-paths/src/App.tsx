import { Button } from "components/Button";
import { sharedValue } from "@shared";
import "chart.js/auto";

export function App() {
  return `${Button}-${sharedValue}`;
}
