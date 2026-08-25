import assert from "node:assert/strict";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { stageUpdaterFeed } from "./stage-updater-feed.mjs";

const platformAssets = {
  "windows-x86_64": "MetaClean_1.2.3_x64-setup.exe",
  "windows-i686": "MetaClean_1.2.3_x86-setup.exe",
  "darwin-aarch64": "MetaClean_1.2.3_aarch64.app.tar.gz",
  "darwin-x86_64": "MetaClean_1.2.3_x64.app.tar.gz",
  "linux-x86_64": "MetaClean_1.2.3_amd64.AppImage",
};

function signatureFor(asset) {
  return Buffer.from(
    `untrusted comment: signature from tauri secret key\nRWRfixture\ntrusted comment: timestamp:1786989648\tfile:${asset}\nRWRfixtureproof\n`,
    "utf8",
  ).toString("base64");
}

function manifest() {
  return {
    version: "1.2.3",
    notes: "Verified update",
    pub_date: "2026-08-25T00:00:00.000Z",
    platforms: Object.fromEntries(Object.entries(platformAssets).map(([platform, asset]) => [platform, {
      signature: signatureFor(asset),
      url: `https://github.com/Moresyl/metaclean/releases/download/v1.2.3/${asset}`,
    }])),
  };
}

test("stages the exact signed release manifest for the Pages fallback", async () => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "metaclean-update-feed-"));
  try {
    const sourcePath = path.join(directory, "source.json");
    const outputPath = path.join(directory, "site", "latest.json");
    await writeFile(sourcePath, JSON.stringify(manifest()));
    await stageUpdaterFeed({ sourcePath, tag: "v1.2.3", repository: "Moresyl/metaclean", outputPath });
    assert.deepEqual(JSON.parse(await readFile(outputPath, "utf8")), manifest());
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("rejects stale, unsigned and unofficial fallback manifests", async () => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "metaclean-update-feed-invalid-"));
  const sourcePath = path.join(directory, "latest.json");
  const options = { sourcePath, tag: "v1.2.3", repository: "Moresyl/metaclean", outputPath: path.join(directory, "site.json") };
  try {
    await writeFile(sourcePath, JSON.stringify({ ...manifest(), version: "1.2.2" }));
    await assert.rejects(stageUpdaterFeed(options), /does not match release/u);
    const unsigned = manifest();
    unsigned.platforms["windows-x86_64"].signature = "";
    await writeFile(sourcePath, JSON.stringify(unsigned));
    await assert.rejects(stageUpdaterFeed(options), /signature is empty/u);
    const unofficial = manifest();
    unofficial.platforms["linux-x86_64"].url = "https://example.com/MetaClean.AppImage";
    await writeFile(sourcePath, JSON.stringify(unofficial));
    await assert.rejects(stageUpdaterFeed(options), /not the official release asset/u);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
