import assert from "node:assert/strict";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import {
  validateUpdateInformation,
  validateZsyncContent,
  verifyAppImageUpdate
} from "./verify-appimage-update.mjs";

const updateInformation = "gh-releases-zsync|Moresyl|metaclean|latest|MetaClean_*_amd64.AppImage.zsync";
const appImageName = "MetaClean_1.2.3_amd64.AppImage";

test("validates GitHub Releases update information and zsync headers", () => {
  validateUpdateInformation(`String dump of section '.upd_info':\n  [ 0] ${updateInformation}\n`, updateInformation);
  validateZsyncContent(`zsync: 0.6.2\nFilename: ${appImageName}\nURL: ${appImageName}\n`, appImageName);
  assert.throws(() => validateUpdateInformation("missing", updateInformation), /\.upd_info/u);
  assert.throws(() => validateUpdateInformation(updateInformation, "zsync|https://example.test/file.zsync"), /Invalid/u);
  assert.throws(() => validateZsyncContent("Filename: Other.AppImage\nURL: Other.AppImage\n", appImageName), /does not target/u);
  assert.throws(() => validateZsyncContent(`Filename: ${appImageName}\nURL: Other.AppImage\n`, appImageName), /does not download/u);
});

test("verifies a matched AppImage and zsync pair with an injected ELF inspector", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "metaclean-appimage-"));
  try {
    const bundle = path.join(root, "bundle", "appimage");
    await mkdir(path.join(bundle, "MetaClean.AppDir"), { recursive: true });
    await writeFile(path.join(bundle, appImageName), "appimage");
    await writeFile(
      path.join(bundle, `${appImageName}.zsync`),
      `zsync: 0.6.2\nFilename: ${appImageName}\nURL: https://example.test/${appImageName}\n`
    );
    await writeFile(path.join(bundle, "MetaClean.AppDir", "ignored.AppImage"), "internal");
    let inspectedPath;
    const result = await verifyAppImageUpdate(root, updateInformation, async (file) => {
      inspectedPath = file;
      return updateInformation;
    });
    assert.equal(inspectedPath, path.join(bundle, appImageName));
    assert.deepEqual(result, [appImageName, `${appImageName}.zsync`]);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("rejects missing, duplicate and empty AppImage delta artifacts", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "metaclean-appimage-invalid-"));
  const inspect = async () => updateInformation;
  try {
    await assert.rejects(verifyAppImageUpdate(root, updateInformation, inspect), /found 0/u);
    await writeFile(path.join(root, appImageName), "appimage");
    await assert.rejects(verifyAppImageUpdate(root, updateInformation, inspect), /Missing/u);
    await writeFile(path.join(root, `${appImageName}.zsync`), "");
    await assert.rejects(verifyAppImageUpdate(root, updateInformation, inspect), /non-empty/u);
    await writeFile(path.join(root, `${appImageName}.zsync`), `Filename: ${appImageName}\nURL: ${appImageName}\n`);
    await writeFile(path.join(root, "Other.AppImage"), "other");
    await assert.rejects(verifyAppImageUpdate(root, updateInformation, inspect), /found 2/u);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
