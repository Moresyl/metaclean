import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { collectReleaseAssets } from "./collect-release-assets.mjs";

test("collects one non-empty package per expected extension", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "metaclean-assets-"));
  try {
    const bundle = path.join(root, "bundle");
    const output = path.join(root, "output");
    await mkdir(path.join(bundle, "nsis"), { recursive: true });
    await mkdir(path.join(bundle, "msi"), { recursive: true });
    await writeFile(path.join(bundle, "nsis", "MetaClean-setup.EXE"), "exe");
    await writeFile(path.join(bundle, "msi", "MetaClean.msi"), "msi");
    assert.deepEqual(await collectReleaseAssets(bundle, output, ["exe", ".msi"]), ["MetaClean-setup.EXE", "MetaClean.msi"]);
    assert.equal(await readFile(path.join(output, "MetaClean.msi"), "utf8"), "msi");
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("ignores application bundle contents when selecting final packages", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "metaclean-assets-appimage-"));
  try {
    const bundle = path.join(root, "bundle");
    const output = path.join(root, "output");
    await mkdir(path.join(bundle, "appimage", "MetaClean.AppDir"), { recursive: true });
    await writeFile(path.join(bundle, "appimage", "MetaClean.AppImage"), "appimage");
    await writeFile(path.join(bundle, "appimage", "MetaClean.AppDir", "internal.AppImage"), "internal");
    assert.deepEqual(await collectReleaseAssets(bundle, output, ["AppImage"]), ["MetaClean.AppImage"]);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("rejects missing, duplicate and empty packages", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "metaclean-assets-invalid-"));
  try {
    await writeFile(path.join(root, "one.dmg"), "one");
    await writeFile(path.join(root, "two.dmg"), "two");
    await assert.rejects(collectReleaseAssets(root, path.join(root, "out"), ["dmg"]), /found 2/u);
    await assert.rejects(collectReleaseAssets(root, path.join(root, "out"), ["rpm"]), /found 0/u);
    await assert.rejects(collectReleaseAssets(root, path.join(root, "out"), ["dmg", "dmg"]), /unique/u);
    await writeFile(path.join(root, "empty.rpm"), "");
    await assert.rejects(collectReleaseAssets(root, path.join(root, "out"), ["rpm"]), /is empty/u);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
