import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile, readdir } from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { validateUpdaterManifest } from "./generate-updater-manifest.mjs";

export async function verifyChecksums(directory, manifest) {
  const files = (await readdir(directory)).sort();
  const listed = new Set();
  for (const line of manifest.trim().split(/\r?\n/u)) {
    const match = /^([a-f0-9]{64})  (.+)$/u.exec(line);
    assert.ok(match, "Malformed checksum entry");
    const [, expected, name] = match;
    assert.ok(!/[\\/]/u.test(name) && name !== "." && name !== "..", "Unsafe checksum path");
    assert.ok(!listed.has(name), "Duplicate checksum entry");
    listed.add(name);
    const bytes = await readFile(path.join(directory, name));
    assert.ok(bytes.length > 0, `Empty asset: ${name}`);
    assert.equal(createHash("sha256").update(bytes).digest("hex"), expected, `Checksum mismatch: ${name}`);
  }
  assert.deepEqual([...listed].sort(), files.filter((name) => name !== "SHASUMS256.txt"), "Incomplete checksum inventory");
  return listed;
}

export async function verifyPublishedAssets(directory, version, release) {
  assert.equal(release.isDraft, false);
  assert.equal(release.isPrerelease, false);
  const files = (await readdir(directory)).sort();
  assert.deepEqual(files, release.assets.map((asset) => asset.name).sort(), "Incomplete release download");
  const listed = await verifyChecksums(directory, await readFile(path.join(directory, "SHASUMS256.txt"), "utf8"));
  const update = JSON.parse(await readFile(path.join(directory, "latest.json"), "utf8"));
  validateUpdaterManifest(update, { version, repository: "Moresyl/metaclean" });
  for (const entry of Object.values(update.platforms)) {
    const name = decodeURIComponent(new URL(entry.url).pathname.split("/").at(-1));
    assert.ok(listed.has(name), `Missing updater asset: ${name}`);
    assert.equal((await readFile(path.join(directory, `${name}.sig`), "utf8")).trim(), entry.signature.trim());
  }
  return { version, verifiedAssets: listed.size, updaterPlatforms: Object.keys(update.platforms) };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [directory, version, releasePath] = process.argv.slice(2);
  if (!directory || !version || !releasePath) throw new Error("Expected asset directory, version and release JSON");
  console.log(await verifyPublishedAssets(directory, version, JSON.parse(await readFile(releasePath, "utf8"))));
}
