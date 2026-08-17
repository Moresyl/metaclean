#!/usr/bin/env bash
set -euo pipefail

bundle_directory="$1"
deb_files=("$bundle_directory"/deb/*.deb)
if [[ ${#deb_files[@]} -ne 1 || ! -f "${deb_files[0]}" ]]; then
  echo "Expected exactly one DEB package" >&2
  exit 1
fi

package_name="$(dpkg-deb --field "${deb_files[0]}" Package)"
cleanup() {
  sudo apt-get remove -y "$package_name" >/dev/null 2>&1 || true
}
trap cleanup EXIT

sudo apt-get install -y "${deb_files[0]}"
binary="$(dpkg -L "$package_name" | awk '/\/usr\/bin\// { print; exit }')"
if [[ -z "$binary" || ! -x "$binary" ]]; then
  echo "Installed DEB does not expose an executable under /usr/bin" >&2
  exit 1
fi

set +e
timeout --kill-after=2s 8s xvfb-run --auto-servernum "$binary"
status=$?
set -e
if [[ $status -ne 124 ]]; then
  echo "Installed MetaClean did not remain active for the smoke window (exit $status)" >&2
  exit 1
fi
echo "Installed DEB launched for eight seconds and will be removed."
