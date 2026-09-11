import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import process from "node:process";

const result = spawnSync(
  process.platform === "win32" ? "cargo.exe" : "cargo",
  ["test", "--manifest-path", "src-tauri/Cargo.toml", "--release", "--lib", "engine::tests::benchmark", "--", "--ignored", "--nocapture"],
  { cwd: fileURLToPath(new URL("..", import.meta.url)), encoding: "utf8", stdio: "inherit" },
);

if (result.error) throw result.error;
process.exit(result.status ?? 1);
