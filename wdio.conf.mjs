import { existsSync } from "node:fs";
import { resolve } from "node:path";

const executable = process.platform === "win32" ? "metaclean.exe" : "metaclean";
const appBinaryPath = process.env.METACLEAN_E2E_BINARY
  ? resolve(process.env.METACLEAN_E2E_BINARY)
  : resolve("src-tauri", "target", "debug", executable);

if (!existsSync(appBinaryPath)) {
  throw new Error(
    `MetaClean E2E binary not found at ${appBinaryPath}. Run \`pnpm test:e2e:build\` first.`,
  );
}

export const config = {
  runner: "local",
  specs: ["./e2e/**/*.e2e.mjs"],
  maxInstances: 1,
  capabilities: [{
    browserName: "tauri",
    "tauri:options": { application: appBinaryPath },
  }],
  services: [["@wdio/tauri-service", {
    appBinaryPath,
    driverProvider: "embedded",
    startTimeout: 120_000,
    statusPollTimeout: 10_000,
  }]],
  logLevel: "warn",
  bail: 0,
  waitforTimeout: 15_000,
  connectionRetryTimeout: 120_000,
  connectionRetryCount: 2,
  framework: "mocha",
  reporters: ["spec"],
  mochaOpts: { ui: "bdd", timeout: 90_000 },
};
