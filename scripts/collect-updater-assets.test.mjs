import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { collectUpdaterAssets } from "./collect-updater-assets.mjs";

async function fixture(suffix, platform) {
  const root = await mkdtemp(path.join(os.tmpdir(), "metaclean-updater-assets-"));
  const bundle = path.join(root, "bundle");
  const output = path.join(root, "output");
  await mkdir(bundle);
  const packagePath = path.join(bundle, `generated${suffix}`);
  await writeFile(packagePath, `${platform}-package`);
  await writeFile(`${packagePath}.sig`, `${platform}-signature`);
  return { root, bundle, output };
}

for (const [platform, suffix, packageName] of [
  ["windows-x86_64", ".exe", "generated.exe"],
  ["linux-x86_64", ".AppImage", "generated.AppImage"],
  ["darwin-aarch64", ".app.tar.gz", "MetaClean_1.2.3_aarch64.app.tar.gz"],
  ["darwin-x86_64", ".app.tar.gz", "MetaClean_1.2.3_x64.app.tar.gz"]
]) {
  test(`collects and normalizes ${platform} updater assets`, async () => {
    const { root, bundle, output } = await fixture(suffix, platform);
    try {
      assert.deepEqual(await collectUpdaterAssets(bundle, output, platform, "1.2.3"), [packageName, `${packageName}.sig`]);
      assert.equal(await readFile(path.join(output, packageName), "utf8"), `${platform}-package`);
      assert.equal(await readFile(path.join(output, `${packageName}.sig`), "utf8"), `${platform}-signature`);
    } finally {
      await rm(root, { recursive: true, force: true });
    }
  });
}

test("rejects unsupported, missing, duplicate and empty updater assets", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "metaclean-updater-invalid-"));
  try {
    await assert.rejects(collectUpdaterAssets(root, root, "plan9-x86_64", "1.0.0"), /Unsupported/u);
    await assert.rejects(collectUpdaterAssets(root, root, "windows-x86_64", "latest"), /Invalid updater version/u);
    await assert.rejects(collectUpdaterAssets(root, root, "windows-x86_64", "1.0.0"), /found 0/u);
    await writeFile(path.join(root, "one.exe"), "one");
    await writeFile(path.join(root, "one.exe.sig"), "sig");
    await writeFile(path.join(root, "two.exe"), "two");
    await writeFile(path.join(root, "two.exe.sig"), "sig");
    await assert.rejects(collectUpdaterAssets(root, root, "windows-x86_64", "1.0.0"), /found 2/u);
    await rm(path.join(root, "two.exe"));
    await rm(path.join(root, "two.exe.sig"));
    await writeFile(path.join(root, "one.exe"), "");
    await assert.rejects(collectUpdaterAssets(root, root, "windows-x86_64", "1.0.0"), /is empty/u);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
