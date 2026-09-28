#!/usr/bin/env bash
set -euo pipefail

[[ "${RUNNER_ENVIRONMENT:-}" == github-hosted && -n "${RUNNER_TEMP:-}" ]] || {
  echo 'Requires a disposable GitHub-hosted runner' >&2; exit 1;
}
for version in "$PREVIOUS_VERSION" "$CURRENT_VERSION"; do
  [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo 'Invalid stable version' >&2; exit 1; }
done
dpkg --compare-versions "$PREVIOUS_VERSION" lt "$CURRENT_VERSION"
if dpkg-query -W -f='${Status}' metaclean 2>/dev/null | grep -q 'install ok installed'; then
  echo 'Refusing to replace an existing installation' >&2; exit 1
fi
if pgrep -x metaclean >/dev/null; then
  echo 'Refusing to interrupt an existing application' >&2; exit 1
fi

assets="$(mktemp -d "$RUNNER_TEMP/metaclean-upgrade.XXXXXX")"
for version in "$PREVIOUS_VERSION" "$CURRENT_VERSION"; do
  gh release view "v$version" --json isDraft,isPrerelease | jq -e '.isDraft == false and .isPrerelease == false'
  directory="$assets/$version"
  mkdir "$directory"
  filename="MetaClean_${version}_amd64.deb"
  gh release download "v$version" --dir "$directory" --pattern "$filename" --pattern SHASUMS256.txt
  awk -v name="$filename" '$2 == name { print }' "$directory/SHASUMS256.txt" > "$directory/selected.sha256"
  [[ "$(wc -l < "$directory/selected.sha256")" -eq 1 ]]
  (cd "$directory" && sha256sum --check selected.sha256)
  [[ "$(dpkg-deb --field "$directory/$filename" Package)" == metaclean ]]
  [[ "$(dpkg-deb --field "$directory/$filename" Version)" == "$version" ]]
  [[ "$(dpkg-deb --field "$directory/$filename" Architecture)" == amd64 ]]
done

attempted=false
cleanup() {
  if [[ "$attempted" == true ]]; then sudo apt-get remove -y metaclean; fi
}
trap cleanup EXIT
printf 'expected_version\tinstalled_version\tlaunch_timeout_status\n' > "$RUNNER_TEMP/metaclean-linux-upgrade.tsv"
for version in "$PREVIOUS_VERSION" "$CURRENT_VERSION" "$PREVIOUS_VERSION"; do
  attempted=true
  sudo apt-get install -y --allow-downgrades "$assets/$version/MetaClean_${version}_amd64.deb"
  installed="$(dpkg-query -W -f='${Version}' metaclean)"
  [[ "$installed" == "$version" ]]
  binary="$(dpkg -L metaclean | awk '/^\/usr\/bin\/[^/]+$/ { print; exit }')"
  [[ -n "$binary" && -x "$binary" ]]
  set +e
  timeout --kill-after=2s 8s xvfb-run --auto-servernum "$binary"
  status=$?
  set -e
  [[ "$status" -eq 124 ]] || { echo "Application exited before smoke window: $status" >&2; exit 1; }
  printf '%s\t%s\t%s\n' "$version" "$installed" "$status" >> "$RUNNER_TEMP/metaclean-linux-upgrade.tsv"
  echo "Installed and launched DEB $version"
done
sudo apt-get remove -y metaclean
attempted=false
[[ ! -e "$binary" ]]
if dpkg-query -W -f='${Status}' metaclean 2>/dev/null | grep -q 'install ok installed'; then
  echo 'Package remains installed' >&2; exit 1
fi
echo 'DEB install, upgrade, manual downgrade and removal passed'
