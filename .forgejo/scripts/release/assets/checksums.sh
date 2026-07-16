#!/usr/bin/env sh
set -eu

version=${1:-}
release_root=${2:-}

[ -n "$version" ] || { echo "release version is required" >&2; exit 1; }
[ -n "$release_root" ] || { echo "release root is required" >&2; exit 1; }
[ -d "$release_root" ] || { echo "release root not found: $release_root" >&2; exit 1; }

(
  cd "$release_root"
  for file in sidecar-*.tar.gz sidecar-*.zip; do
    [ -f "$file" ] || continue
    shasum -a 256 "$file"
  done
) > "$release_root/checksums.txt"

test -s "$release_root/checksums.txt" || {
  echo "no release archives found for $version" >&2
  exit 1
}

