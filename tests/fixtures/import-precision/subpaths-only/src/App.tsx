import Card from "@mui/material/Card";
import { parseISO } from "date-fns/parseISO";
import memoize from "lodash/memoize";

export function App() {
  return <div>{[Card, parseISO, memoize].length}</div>;
}
