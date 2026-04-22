import fs from "fs";

export default function App() {
  return <pre>{fs.readFileSync("/etc/hosts", "utf8")}</pre>;
}
