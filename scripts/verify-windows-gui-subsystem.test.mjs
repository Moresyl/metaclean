import assert from "node:assert/strict";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { readPeSubsystem, verifyWindowsGuiSubsystem } from "./verify-windows-gui-subsystem.mjs";

function peFixture({ magic = 0x020b, subsystem = 2 } = {}) {
  const bytes = Buffer.alloc(256);
  bytes.write("MZ", 0, "ascii");
  bytes.writeUInt32LE(0x80, 0x3c);
  bytes.writeUInt32LE(0x00004550, 0x80);
  bytes.writeUInt16LE(magic, 0x80 + 24);
  bytes.writeUInt16LE(subsystem, 0x80 + 24 + 68);
  return bytes;
}

test("reads GUI and console subsystem values from PE32 and PE32+ executables", () => {
  assert.equal(readPeSubsystem(peFixture({ magic: 0x010b })), 2);
  assert.equal(readPeSubsystem(peFixture({ magic: 0x020b, subsystem: 3 })), 3);
});

test("rejects malformed and unsupported PE executables", () => {
  assert.throws(() => readPeSubsystem(Buffer.alloc(2)), /DOS header/u);
  const missingMz = peFixture();
  missingMz.fill(0, 0, 2);
  assert.throws(() => readPeSubsystem(missingMz), /missing MZ/u);
  const missingPe = peFixture();
  missingPe.fill(0, 0x80, 0x84);
  assert.throws(() => readPeSubsystem(missingPe), /missing PE/u);
  assert.throws(() => readPeSubsystem(peFixture({ magic: 0x9999 })), /unsupported/u);
});

test("accepts GUI binaries and rejects console binaries", async () => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "metaclean-pe-"));
  try {
    const binaryPath = path.join(directory, "metaclean.exe");
    await writeFile(binaryPath, peFixture());
    assert.equal(await verifyWindowsGuiSubsystem(binaryPath), 2);
    await writeFile(binaryPath, peFixture({ subsystem: 3 }));
    await assert.rejects(verifyWindowsGuiSubsystem(binaryPath), /would open a console window/u);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
