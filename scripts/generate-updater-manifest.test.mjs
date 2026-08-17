import assert from "node:assert/strict";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { generateUpdaterManifest } from "./generate-updater-manifest.mjs";

const assets = [
  "MetaClean_1.2.3_x64-setup.exe",
  "MetaClean_1.2.3_x86-setup.exe",
  "MetaClean_1.2.3_aarch64.app.tar.gz",
  "MetaClean_1.2.3_x64.app.tar.gz",
  "MetaClean_1.2.3_amd64.AppImage"
];

function signatureFor(asset) {
  return Buffer.from(
    `untrusted comment: signature from tauri secret key\nRWRfixture\ntrusted comment: timestamp:1786989648\tfile:${asset}\nRWRfixtureproof\n`,
    "utf8",
  ).toString("base64");
}

async function createAssets(directory) {
  for (const asset of assets) {
    await writeFile(path.join(directory, asset), "package");
    await writeFile(path.join(directory, `${asset}.sig`), signatureFor(asset));
  }
}

test("generates a complete stable updater manifest for all release targets", async () => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "metaclean-updater-manifest-"));
  const outputPath = path.join(directory, "latest.json");
  try {
    await createAssets(directory);
    const manifest = await generateUpdaterManifest({
      assetDirectory: directory,
      tag: "v1.2.3",
      repository: "Moresyl/metaclean",
      notes: "中文更新\n\nEnglish summary",
      pubDate: "2026-08-18T00:00:00Z",
      outputPath
    });
    assert.equal(manifest.version, "1.2.3");
    assert.equal(manifest.pub_date, "2026-08-18T00:00:00.000Z");
    assert.deepEqual(Object.keys(manifest.platforms), [
      "windows-x86_64", "windows-i686", "darwin-aarch64", "darwin-x86_64", "linux-x86_64"
    ]);
    assert.equal(manifest.platforms["windows-x86_64"].signature, signatureFor("MetaClean_1.2.3_x64-setup.exe"));
    assert.equal(manifest.platforms["darwin-aarch64"].url,
      "https://github.com/Moresyl/metaclean/releases/download/v1.2.3/MetaClean_1.2.3_aarch64.app.tar.gz");
    assert.deepEqual(JSON.parse(await readFile(outputPath, "utf8")), manifest);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("rejects invalid metadata and incomplete updater assets", async () => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "metaclean-updater-manifest-invalid-"));
  const options = {
    assetDirectory: directory,
    tag: "v1.2.3",
    repository: "Moresyl/metaclean",
    notes: "notes",
    pubDate: "2026-08-18T00:00:00Z",
    outputPath: path.join(directory, "latest.json")
  };
  try {
    await assert.rejects(generateUpdaterManifest({ ...options, tag: "latest" }), /Invalid release tag/u);
    await assert.rejects(generateUpdaterManifest({ ...options, repository: "https:\/\/github.com/a/b" }), /Invalid repository/u);
    await assert.rejects(generateUpdaterManifest({ ...options, pubDate: "never" }), /Invalid publication date/u);
    await assert.rejects(generateUpdaterManifest({ ...options, notes: "" }), /must not be empty/u);
    await assert.rejects(generateUpdaterManifest(options), /Missing updater package/u);
    await createAssets(directory);
    await writeFile(path.join(directory, `${assets[0]}.sig`), "");
    await assert.rejects(generateUpdaterManifest(options), /signature is empty/u);
    await writeFile(path.join(directory, `${assets[0]}.sig`), "not-base64!");
    await assert.rejects(generateUpdaterManifest(options), /not valid base64/u);
    await writeFile(path.join(directory, `${assets[0]}.sig`), Buffer.from("not minisign", "utf8").toString("base64"));
    await assert.rejects(generateUpdaterManifest(options), /not a Tauri minisign payload/u);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
