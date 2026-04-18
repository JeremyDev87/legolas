#!/usr/bin/env node

import { missingBinaryMessage, resolveCurrentHostSupport } from "./platform-support.js";

const hostSupport = resolveCurrentHostSupport();

if (!hostSupport.supported) {
  console.error(missingBinaryMessage(hostSupport.hostKey));
  process.exit(1);
}
