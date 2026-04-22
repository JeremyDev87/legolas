import fs from "node:fs";

export default function App() {
  return <pre>{fs.readFileSync("/etc/hosts", "utf8")}</pre>;
}
