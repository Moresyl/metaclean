import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

test("Windows preflight is a non-publishing candidate gate", async () => {
  const script = await readFile(new URL("./preflight-windows.ps1", import.meta.url), "utf8");
  const msiSmoke = await readFile(new URL("./smoke-windows-msi.ps1", import.meta.url), "utf8");
  const contextMenuSmoke = await readFile(new URL("./smoke-windows-context-menu.ps1", import.meta.url), "utf8");
  assert.match(script, /tauri.*build.*--debug/iu);
  assert.match(script, /package-windows-portable\.ps1/u);
  assert.match(script, /smoke-windows-installer\.ps1/u);
  assert.match(script, /smoke-windows-msi\.ps1/u);
  assert.match(script, /smoke-windows-portable\.ps1/u);
  assert.match(script, /smoke-windows-context-menu\.ps1/u);
  assert.doesNotMatch(script, /gh\s+release\s+create/iu);
  assert.doesNotMatch(script, /git\s+push/iu);
  assert.match(msiSmoke, /Refusing to replace an existing MetaClean installation/u);
  assert.ok(
    msiSmoke.lastIndexOf("Remove-Item -LiteralPath $logRoot") > msiSmoke.indexOf("MSI registration remains after uninstall"),
    "successful cleanup must happen after uninstall validation so failure logs survive",
  );
  assert.match(contextMenuSmoke, /SUPPORTED_EXTENSIONS/u);
  assert.match(contextMenuSmoke, /Refusing to replace.*existing MetaClean Explorer/iu);
  assert.match(contextMenuSmoke, /finally/u);
  assert.match(contextMenuSmoke, /--remove-context-menu/u);
});
