#!/usr/bin/env bash
set -euo pipefail

[[ "${RUNNER_ENVIRONMENT:-}" == github-hosted && -n "${RUNNER_TEMP:-}" ]] || {
  echo 'Requires a disposable GitHub-hosted runner' >&2; exit 1;
}

root="$(mktemp -d "$RUNNER_TEMP/metaclean-storage.XXXXXX")"
volume="$root/volume"
evidence="$RUNNER_TEMP/metaclean-storage-evidence"
mkdir "$volume" "$evidence"
cleanup() {
  if mountpoint -q "$volume"; then sudo umount "$volume"; fi
}
trap cleanup EXIT
sudo mount -t tmpfs -o size=16m tmpfs "$volume"
sudo chown "$(id -u):$(id -g)" "$volume"
printf 'before\342\200\213after\n' > "$volume/sample.txt"
set +e
dd if=/dev/zero of="$volume/filler.bin" bs=64K 2> "$evidence/fill.log"
fill_status=$?
set -e
[[ "$fill_status" -ne 0 ]]
export METACLEAN_STORAGE_SAMPLE_DIR="$volume"
for failure in full readonly; do
  if [[ "$failure" == readonly ]]; then
    rm -- "$volume/filler.bin"
    sudo mount -o remount,ro "$volume"
  fi
  findmnt --target "$volume" > "$evidence/$failure-mount.txt"
  METACLEAN_STORAGE_FAILURE="$failure" cargo test --manifest-path src-tauri/Cargo.toml \
    --lib fails_safely_on_external_storage_failure -- --ignored --nocapture \
    | tee "$evidence/$failure.log"
  grep -q '1 passed; 0 failed' "$evidence/$failure.log"
done
