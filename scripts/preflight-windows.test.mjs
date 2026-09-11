import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

test("Windows preflight is a non-publishing candidate gate", async () => {
  const script = await readFile(new URL("./preflight-windows.ps1", import.meta.url), "utf8");
  assert.match(script, /tauri.*build.*--debug/iu);
  assert.match(script, /package-windows-portable\.ps1/u);
  assert.match(script, /smoke-windows-installer\.ps1/u);
  assert.match(script, /smoke-windows-msi\.ps1/u);
  assert.match(script, /smoke-windows-portable\.ps1/u);
  assert.doesNotMatch(script, /gh\s+release\s+create/iu);
  assert.doesNotMatch(script, /git\s+push/iu);
});
