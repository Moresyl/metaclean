#!/usr/bin/env bash
set -euo pipefail

[[ "${RUNNER_ENVIRONMENT:-}" == github-hosted && -n "${RUNNER_TEMP:-}" ]] || {
  echo 'Requires a disposable GitHub-hosted runner' >&2; exit 1;
}
for version in "$PREVIOUS_VERSION" "$CURRENT_VERSION"; do
  [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo 'Invalid stable version' >&2; exit 1; }
done
[[ "$PREVIOUS_VERSION" != "$CURRENT_VERSION" ]]
case "$PACKAGE_ARCHITECTURE" in
  aarch64) expected_arch=arm64 ;;
  x64) expected_arch=x86_64 ;;
  *) echo 'Unsupported package architecture' >&2; exit 1 ;;
esac

workspace="$(mktemp -d "$RUNNER_TEMP/metaclean-replacement.XXXXXX")"
mount_point="$workspace/mount"
app_path="$workspace/installed/MetaClean.app"
mkdir -p "$mount_point" "$workspace/installed"
mounted=false
app_pid=''
cleanup() {
  if [[ -n "$app_pid" ]]; then kill "$app_pid" 2>/dev/null || true; wait "$app_pid" 2>/dev/null || true; fi
  if [[ "$mounted" == true ]]; then
    hdiutil detach "$mount_point" -quiet || return 1
  fi
  rm -rf "$workspace"
}
trap cleanup EXIT

for version in "$PREVIOUS_VERSION" "$CURRENT_VERSION"; do
  gh release view "v$version" --json isDraft,isPrerelease | jq -e '.isDraft == false and .isPrerelease == false'
  directory="$workspace/$version"
  mkdir "$directory"
  filename="MetaClean_${version}_${PACKAGE_ARCHITECTURE}.dmg"
  gh release download "v$version" --dir "$directory" --pattern "$filename" --pattern SHASUMS256.txt
  awk -v name="$filename" '$2 == name { print }' "$directory/SHASUMS256.txt" > "$directory/selected.sha256"
  [[ "$(wc -l < "$directory/selected.sha256")" -eq 1 ]]
  (cd "$directory" && shasum -a 256 --check selected.sha256)
done

printf 'expected_version\tinstalled_version\tbundle_id\tpackage_arch\trunner_arch\tlaunch_seconds\n' > "$RUNNER_TEMP/metaclean-macos-upgrade.tsv"
for version in "$PREVIOUS_VERSION" "$CURRENT_VERSION" "$PREVIOUS_VERSION"; do
  hdiutil attach "$workspace/$version/MetaClean_${version}_${PACKAGE_ARCHITECTURE}.dmg" -nobrowse -readonly -mountpoint "$mount_point" -quiet
  mounted=true
  bundles=("$mount_point"/*.app)
  [[ ${#bundles[@]} -eq 1 && -d "${bundles[0]}" ]]
  # Only replace the bundle owned by this test, never /Applications.
  rm -rf "$app_path"
  ditto "${bundles[0]}" "$app_path"
  hdiutil detach "$mount_point" -quiet
  mounted=false
  plist="$app_path/Contents/Info.plist"
  installed="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$plist")"
  bundle_id="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$plist")"
  [[ "$installed" == "$version" && "$bundle_id" == com.moresl.metaclean ]]
  binaries=("$app_path/Contents/MacOS"/*)
  [[ ${#binaries[@]} -eq 1 && -x "${binaries[0]}" ]]
  [[ "$(lipo -archs "${binaries[0]}")" == "$expected_arch" ]]
  "${binaries[0]}" &
  app_pid=$!
  sleep 6
  kill -0 "$app_pid"
  kill "$app_pid"
  wait "$app_pid" 2>/dev/null || true
  app_pid=''
  printf '%s\t%s\t%s\t%s\t%s\t6\n' "$version" "$installed" "$bundle_id" "$expected_arch" "$(uname -m)" >> "$RUNNER_TEMP/metaclean-macos-upgrade.tsv"
  echo "Replaced and launched DMG application $version ($expected_arch)"
done
rm -rf "$app_path"
[[ ! -e "$app_path" ]]
echo 'DMG copy, manual replacement, rollback and removal passed'
