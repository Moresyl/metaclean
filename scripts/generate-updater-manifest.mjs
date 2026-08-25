import { readdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";

const PLATFORM_ASSETS = {
  "windows-x86_64": (version) => `MetaClean_${version}_x64-setup.exe`,
  "windows-i686": (version) => `MetaClean_${version}_x86-setup.exe`,
  "darwin-aarch64": (version) => `MetaClean_${version}_aarch64.app.tar.gz`,
  "darwin-x86_64": (version) => `MetaClean_${version}_x64.app.tar.gz`,
  "linux-x86_64": (version) => `MetaClean_${version}_amd64.AppImage`
};

function validateRepository(repository) {
  if (!/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/u.test(repository)) throw new Error(`Invalid repository: ${repository}`);
}

function validateSignature(signature, signatureAsset) {
  if (!/^[A-Za-z0-9+/]+={0,2}$/u.test(signature)) {
    throw new Error(`Updater signature is not valid base64: ${signatureAsset}`);
  }
  const decoded = Buffer.from(signature, "base64").toString("utf8");
  if (!decoded.startsWith("untrusted comment: signature from tauri secret key\n")
      || !decoded.includes("\ntrusted comment: timestamp:")) {
    throw new Error(`Updater signature is not a Tauri minisign payload: ${signatureAsset}`);
  }
}

export function validateUpdaterManifest(manifest, { version, repository }) {
  validateRepository(repository);
  if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/u.test(version)) throw new Error(`Invalid updater version: ${version}`);
  if (!manifest || typeof manifest !== "object") throw new Error("Updater manifest must be an object");
  if (manifest.version !== version) throw new Error(`Updater manifest version ${String(manifest.version)} does not match release ${version}`);
  if (typeof manifest.notes !== "string" || manifest.notes.trim().length === 0) throw new Error("Updater notes must not be empty");
  if (typeof manifest.pub_date !== "string" || Number.isNaN(new Date(manifest.pub_date).valueOf())) throw new Error("Updater publication date is invalid");

  const platforms = Object.entries(manifest.platforms ?? {});
  if (platforms.length !== Object.keys(PLATFORM_ASSETS).length) throw new Error("Updater manifest has an incomplete platform matrix");
  for (const [platform, resolveAsset] of Object.entries(PLATFORM_ASSETS)) {
    const artifact = manifest.platforms?.[platform];
    if (!artifact || typeof artifact !== "object") throw new Error(`Updater manifest is missing ${platform}`);
    const asset = resolveAsset(version);
    const expectedUrl = `https://github.com/${repository}/releases/download/v${version}/${encodeURIComponent(asset)}`;
    if (artifact.url !== expectedUrl) throw new Error(`Updater URL is not the official release asset for ${platform}`);
    if (typeof artifact.signature !== "string" || !artifact.signature.trim()) throw new Error(`Updater signature is empty for ${platform}`);
    validateSignature(artifact.signature.trim(), `${asset}.sig`);
  }
  return manifest;
}

export async function generateUpdaterManifest({ assetDirectory, tag, repository, notes, pubDate, outputPath }) {
  if (!/^v\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/u.test(tag)) throw new Error(`Invalid release tag: ${tag}`);
  validateRepository(repository);
  const version = tag.slice(1);
  const normalizedDate = new Date(pubDate);
  if (Number.isNaN(normalizedDate.valueOf())) throw new Error(`Invalid publication date: ${pubDate}`);
  if (typeof notes !== "string" || notes.trim().length === 0) throw new Error("Updater notes must not be empty");

  const availableAssets = new Set((await readdir(assetDirectory, { withFileTypes: true }))
    .filter((entry) => entry.isFile())
    .map((entry) => entry.name));
  const platforms = {};
  for (const [platform, resolveAsset] of Object.entries(PLATFORM_ASSETS)) {
    const asset = resolveAsset(version);
    const signatureAsset = `${asset}.sig`;
    if (!availableAssets.has(asset)) throw new Error(`Missing updater package for ${platform}: ${asset}`);
    if (!availableAssets.has(signatureAsset)) throw new Error(`Missing updater signature for ${platform}: ${signatureAsset}`);
    const signature = (await readFile(path.join(assetDirectory, signatureAsset), "utf8")).trim();
    if (!signature) throw new Error(`Updater signature is empty for ${platform}: ${signatureAsset}`);
    validateSignature(signature, signatureAsset);
    platforms[platform] = {
      signature,
      url: `https://github.com/${repository}/releases/download/${encodeURIComponent(tag)}/${encodeURIComponent(asset)}`
    };
  }

  const manifest = {
    version,
    notes: notes.trim(),
    pub_date: normalizedDate.toISOString(),
    platforms
  };
  validateUpdaterManifest(manifest, { version, repository });
  await writeFile(outputPath, `${JSON.stringify(manifest, null, 2)}\n`, "utf8");
  return manifest;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [assetDirectory, tag, repository, notesPath, pubDate, outputPath] = process.argv.slice(2);
  if (!assetDirectory || !tag || !repository || !notesPath || !pubDate || !outputPath) {
    throw new Error("Usage: generate-updater-manifest.mjs <asset-directory> <tag> <repository> <notes-file> <pub-date> <output-file>");
  }
  const manifest = await generateUpdaterManifest({
    assetDirectory,
    tag,
    repository,
    notes: await readFile(notesPath, "utf8"),
    pubDate,
    outputPath
  });
  console.log(`Generated signed updater manifest ${path.basename(outputPath)} for ${Object.keys(manifest.platforms).length} platforms.`);
}
