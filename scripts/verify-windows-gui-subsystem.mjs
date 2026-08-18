import { readFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";

const PE_SIGNATURE = 0x00004550;
const PE32_MAGIC = 0x010b;
const PE32_PLUS_MAGIC = 0x020b;
const WINDOWS_GUI_SUBSYSTEM = 2;

function requireRange(bytes, offset, size, label) {
  if (!Number.isInteger(offset) || offset < 0 || offset + size > bytes.length) {
    throw new Error(`Invalid PE executable: ${label} is outside the file`);
  }
}

export function readPeSubsystem(input) {
  const bytes = Buffer.from(input);
  requireRange(bytes, 0, 0x40, "DOS header");
  if (bytes[0] !== 0x4d || bytes[1] !== 0x5a) {
    throw new Error("Invalid PE executable: missing MZ header");
  }

  const peOffset = bytes.readUInt32LE(0x3c);
  requireRange(bytes, peOffset, 24, "PE header");
  if (bytes.readUInt32LE(peOffset) !== PE_SIGNATURE) {
    throw new Error("Invalid PE executable: missing PE signature");
  }

  const optionalHeaderOffset = peOffset + 24;
  requireRange(bytes, optionalHeaderOffset, 70, "optional header");
  const magic = bytes.readUInt16LE(optionalHeaderOffset);
  if (magic !== PE32_MAGIC && magic !== PE32_PLUS_MAGIC) {
    throw new Error(`Invalid PE executable: unsupported optional-header magic 0x${magic.toString(16)}`);
  }
  return bytes.readUInt16LE(optionalHeaderOffset + 68);
}

export async function verifyWindowsGuiSubsystem(binaryPath) {
  const subsystem = readPeSubsystem(await readFile(binaryPath));
  if (subsystem !== WINDOWS_GUI_SUBSYSTEM) {
    throw new Error(`Expected Windows GUI subsystem (2), found ${subsystem}; this build would open a console window`);
  }
  return subsystem;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [binaryPath] = process.argv.slice(2);
  if (!binaryPath) throw new Error("Usage: verify-windows-gui-subsystem.mjs <exe-path>");
  await verifyWindowsGuiSubsystem(binaryPath);
  console.log(`Verified Windows GUI subsystem: ${binaryPath}`);
}
