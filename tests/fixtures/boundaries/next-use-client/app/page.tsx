/* eslint-disable react/no-danger */ "use client";

import fs from "node:fs";

export default function Page() {
  return <pre>{fs.readFileSync("/etc/hosts", "utf8")}</pre>;
}
