import { Button } from "@mui/material";
import Card from "@mui/material/Card";
import { formatDistanceToNow } from "date-fns";
import { parseISO } from "date-fns/parseISO";
import { chunk } from "lodash";
import memoize from "lodash/memoize";

export function App() {
  const summary = formatDistanceToNow(parseISO("2024-01-01"));
  return <div>{chunk([Button, Card, memoize, summary], 2).length}</div>;
}
