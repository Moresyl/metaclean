import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { verifyChecksums, verifyPublishedAssets } from "./verify-published-assets.mjs";

test("checks a complete published matrix and rejects mismatched signature sidecars", async () => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "metaclean-matrix-"));
  try {
    const version = "0.10.0";
    const signature = Buffer.from("untrusted comment: signature from tauri secret key\npayload\ntrusted comment: timestamp:1\nproof").toString("base64");
    const names = {
      "windows-x86_64": `MetaClean_${version}_x64-setup.exe`,
      "windows-i686": `MetaClean_${version}_x86-setup.exe`,
      "darwin-aarch64": `MetaClean_${version}_aarch64.app.tar.gz`,
      "darwin-x86_64": `MetaClean_${version}_x64.app.tar.gz`,
      "linux-x86_64": `MetaClean_${version}_amd64.AppImage`,
    };
    const platforms = Object.fromEntries(Object.entries(names).map(([platform, name]) => [platform, {
      url: `https://github.com/Moresyl/metaclean/releases/download/v${version}/${name}`, signature,
    }]));
    const assets = { "latest.json": JSON.stringify({ version, notes: "Concrete release updates", pub_date: "2026-09-28T00:00:00Z", platforms }) };
    for (const name of Object.values(names)) { assets[name] = "package"; assets[`${name}.sig`] = signature; }
    async function save() {
      for (const [name, bytes] of Object.entries(assets)) await writeFile(path.join(directory, name), bytes);
      await writeFile(path.join(directory, "SHASUMS256.txt"), Object.entries(assets).map(([name, bytes]) => `${createHash("sha256").update(bytes).digest("hex")}  ${name}`).join("\n"));
    }
    await save();
    const release = { isDraft: false, isPrerelease: false, assets: [...Object.keys(assets), "SHASUMS256.txt"].map((name) => ({ name })) };
    assert.equal((await verifyPublishedAssets(directory, version, release)).verifiedAssets, 11);
    assets[`${names["windows-x86_64"]}.sig`] = "mismatched";
    await save();
    await assert.rejects(verifyPublishedAssets(directory, version, release));
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("verifies bytes and rejects corruption, missing inventory and unsafe names", async () => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "metaclean-published-"));
  const digest = createHash("sha256").update("package").digest("hex");
  const manifest = `${digest}  package.zip\n`;
  try {
    await writeFile(path.join(directory, "package.zip"), "package");
    assert.deepEqual([...await verifyChecksums(directory, manifest)], ["package.zip"]);
    await assert.rejects(verifyChecksums(directory, manifest + manifest), /Duplicate/);
    for (const name of ["../outside", "..\\outside", ".", ".."]) {
      await assert.rejects(verifyChecksums(directory, `${digest}  ${name}`), /Unsafe/);
    }
    await assert.rejects(verifyChecksums(directory, ""), /Malformed/);
    await writeFile(path.join(directory, "extra.zip"), "extra");
    await assert.rejects(verifyChecksums(directory, manifest), /Incomplete/);
    await writeFile(path.join(directory, "package.zip"), "changed");
    await assert.rejects(verifyChecksums(directory, manifest), /Checksum mismatch/);
    await writeFile(path.join(directory, "package.zip"), "");
    await assert.rejects(verifyChecksums(directory, manifest), /Empty/);
    await assert.rejects(verifyPublishedAssets(directory, "0.10.0", { isDraft: true }));
    await assert.rejects(verifyPublishedAssets(directory, "0.10.0", { isDraft: false, isPrerelease: true }));
    await assert.rejects(verifyPublishedAssets(directory, "0.10.0", { isDraft: false, isPrerelease: false, assets: [] }), /Incomplete release/);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
