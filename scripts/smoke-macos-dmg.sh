#!/usr/bin/env bash
set -euo pipefail

bundle_directory="$1"
dmg_files=("$bundle_directory"/dmg/*.dmg)
if [[ ${#dmg_files[@]} -ne 1 || ! -f "${dmg_files[0]}" ]]; then
  echo "Expected exactly one DMG package" >&2
  exit 1
fi

mount_point="$(mktemp -d)"
install_root="$(mktemp -d)"
cleanup() {
  if [[ -n "${app_pid:-}" ]]; then kill "$app_pid" 2>/dev/null || true; fi
  hdiutil detach "$mount_point" -quiet 2>/dev/null || true
  rm -rf "$mount_point" "$install_root"
}
trap cleanup EXIT

hdiutil attach "${dmg_files[0]}" -nobrowse -readonly -mountpoint "$mount_point" -quiet
app_bundle=("$mount_point"/*.app)
if [[ ${#app_bundle[@]} -ne 1 || ! -d "${app_bundle[0]}" ]]; then
  echo "Expected exactly one app bundle in the DMG" >&2
  exit 1
fi
ditto "${app_bundle[0]}" "$install_root/MetaClean.app"
binary=("$install_root/MetaClean.app/Contents/MacOS"/*)
if [[ ${#binary[@]} -ne 1 || ! -x "${binary[0]}" ]]; then
  echo "Installed app bundle does not contain one executable" >&2
  exit 1
fi
"${binary[0]}" &
app_pid=$!
sleep 6
kill -0 "$app_pid"
echo "Copied DMG application launched for six seconds."
