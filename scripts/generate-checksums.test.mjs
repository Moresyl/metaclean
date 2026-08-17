import assert from "node:assert/strict";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { generateChecksums } from "./generate-checksums.mjs";

test("generates a sorted complete manifest without hashing itself", async () => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "metaclean-checksums-"));
  try {
    await writeFile(path.join(directory, "z-package.bin"), "z");
    await writeFile(path.join(directory, "a-package.bin"), "a");
    await writeFile(path.join(directory, "SHASUMS256.txt"), "stale");
    const output = path.join(directory, "SHASUMS256.txt");
    const manifest = await generateChecksums(directory, output);
    assert.equal(manifest, "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb  a-package.bin\n594e519ae499312b29433b7dd8a97ff068defcba9755b6d5d00e84c524d67b06  z-package.bin\n");
    assert.equal(await readFile(output, "utf8"), manifest);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("refuses an empty asset directory", async () => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "metaclean-checksums-empty-"));
  try {
    await assert.rejects(generateChecksums(directory, path.join(directory, "SHASUMS256.txt")), /No release assets/u);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
