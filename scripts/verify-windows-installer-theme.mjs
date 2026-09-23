import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const installerRoot = path.join(root, "src-tauri", "windows", "installer");

function readBitmapHeader(buffer) {
  assert.equal(buffer.toString("ascii", 0, 2), "BM", "installer artwork must be a Windows bitmap");
  const offset = buffer.readUInt32LE(10);
  const width = buffer.readInt32LE(18);
  const height = buffer.readInt32LE(22);
  const bitsPerPixel = buffer.readUInt16LE(28);
  assert.equal(bitsPerPixel, 24, "installer artwork must stay 24-bit for NSIS/WiX compatibility");
  return { offset, width, height, bitsPerPixel };
}

function pixelAt(buffer, header, x, y) {
  const stride = Math.ceil((header.width * 3) / 4) * 4;
  const storedY = header.height - y - 1;
  const offset = header.offset + storedY * stride + x * 3;
  const [blue, green, red] = buffer.subarray(offset, offset + 3);
  return `#${[red, green, blue].map((value) => value.toString(16).padStart(2, "0")).join("")}`;
}

async function readArtwork(name, expectedWidth, expectedHeight) {
  const buffer = await readFile(path.join(installerRoot, name));
  const header = readBitmapHeader(buffer);
  assert.equal(header.width, expectedWidth, `${name} width drifted`);
  assert.equal(header.height, expectedHeight, `${name} height drifted`);
  return { buffer, header };
}

test("keeps native Windows installer artwork compatible and on palette", async () => {
  const sidebar = await readArtwork("nsis-sidebar.bmp", 164, 314);
  const header = await readArtwork("nsis-header.bmp", 150, 57);
  const banner = await readArtwork("wix-banner.bmp", 493, 58);
  const dialog = await readArtwork("wix-dialog.bmp", 493, 312);

  assert.equal(pixelAt(sidebar.buffer, sidebar.header, 0, 0), "#161616");
  assert.equal(pixelAt(header.buffer, header.header, 0, 0), "#161616");
  assert.equal(pixelAt(banner.buffer, banner.header, 0, 0), "#f0f0f0");
  assert.equal(pixelAt(banner.buffer, banner.header, 492, 0), "#161616");
  assert.equal(pixelAt(dialog.buffer, dialog.header, 0, 0), "#161616");
  assert.equal(pixelAt(dialog.buffer, dialog.header, 492, 311), "#f0f0f0");
});

test("wires the branded artwork into install and uninstall bundles", async () => {
  const config = JSON.parse(await readFile(path.join(root, "src-tauri", "tauri.conf.json"), "utf8"));
  const { nsis, wix } = config.bundle.windows;

  assert.deepEqual(
    {
      headerImage: nsis.headerImage,
      sidebarImage: nsis.sidebarImage,
      installerIcon: nsis.installerIcon,
      uninstallerIcon: nsis.uninstallerIcon,
      uninstallerHeaderImage: nsis.uninstallerHeaderImage,
    },
    {
      headerImage: "./windows/installer/nsis-header.bmp",
      sidebarImage: "./windows/installer/nsis-sidebar.bmp",
      installerIcon: "./icons/icon.ico",
      uninstallerIcon: "./icons/icon.ico",
      uninstallerHeaderImage: "./windows/installer/nsis-header.bmp",
    },
  );
  assert.equal(wix.bannerPath, "./windows/installer/wix-banner.bmp");
  assert.equal(wix.dialogImagePath, "./windows/installer/wix-dialog.bmp");
  assert.deepEqual(nsis.languages, ["SimpChinese", "English"], "NSIS must follow the Windows language with an English fallback");
  assert.notEqual(nsis.displayLanguageSelector, true, "the installer must not add a separate language prompt");
});

test("keeps native installer text legible and reuses the chosen language on uninstall", async () => {
  const hooks = await readFile(new URL("../src-tauri/windows/hooks.nsh", import.meta.url), "utf8");
  assert.match(hooks, /SetFont "Microsoft YaHei UI" 8/);
  assert.match(hooks, /!define MUI_WELCOMEPAGE_TITLE_3LINES/);
  assert.match(hooks, /!macro NSIS_HOOK_POSTINSTALL\s+WriteRegStr HKCU "Software\\moresl\\MetaClean" "Installer Language" \$LANGUAGE\s+!macroend/);
});
