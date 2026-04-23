import { debounce } from "lodash";

export function App() {
  return debounce(() => "ready", 100);
}
