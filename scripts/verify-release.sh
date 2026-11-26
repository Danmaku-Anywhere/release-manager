#!/usr/bin/env bash
# Usage: verify-release.sh <assets dir> <version>
# Checks a downloaded release has every bundle, that latest.json points at this
# version's update bundles, and that each update bundle verifies against the
# public key installed apps trust.
set -euo pipefail

dir=$(cd "$1" && pwd)
version=$2
root=$(cd "$(dirname "$0")/.." && pwd)
fail=0

require() {
  local matches
  matches=$(find "$dir" -maxdepth 1 -name "$1" | wc -l | tr -d ' ')
  if [ "$matches" != 1 ]; then
    echo "::error::expected one asset matching $1, found $matches"
    fail=1
  fi
}

require "*_${version}_aarch64.dmg"
require "*_aarch64.app.tar.gz"
require "*_aarch64.app.tar.gz.sig"
require "*_${version}_amd64.AppImage"
require "*_${version}_amd64.AppImage.sig"
require "*_${version}_x64-setup.exe"
require "*_${version}_x64-portable.exe"
require "latest.json"
if [ "$fail" != 0 ]; then
  exit 1
fi

manifest_version=$(node -p "require('$dir/latest.json').version")
if [ "$manifest_version" != "$version" ]; then
  echo "::error::latest.json says $manifest_version, expected $version"
  exit 1
fi

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
node -p "require('$root/src-tauri/tauri.conf.json').plugins.updater.pubkey" |
  base64 -d >"$tmp/pub.key"

for sig in "$dir"/*.sig; do
  bundle=${sig%.sig}
  base64 -d <"$sig" >"$tmp/bundle.minisig"
  if minisign -V -q -p "$tmp/pub.key" -m "$bundle" -x "$tmp/bundle.minisig"; then
    echo "verified $(basename "$bundle")"
  else
    echo "::error::$(basename "$bundle") does not verify against the app's public key"
    fail=1
  fi
done

for platform in darwin-aarch64 linux-x86_64; do
  url=$(node -p "require('$dir/latest.json').platforms['$platform'].url")
  asset=$(basename "$url")
  if [ ! -f "$dir/$(node -p "decodeURIComponent('$asset')")" ]; then
    echo "::error::latest.json $platform points at $asset, which is not in the release"
    fail=1
  fi
done

exit "$fail"
