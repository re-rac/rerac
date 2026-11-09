#!/usr/bin/env bash
# Builds a ReRAC version folder and zips it (launcher contract: docs/plan/launcher_contract.md).
#
#   tools/package/package.sh              release build of `rerac` + `rerac-extract`, then package
#   tools/package/package.sh --no-build   package the binaries already in target/release
#
# Output: dist/rerac-<version>-<os>-<arch>/ and dist/rerac-<version>-<os>-<arch>.zip, holding
#   rerac[.exe]              the runtime (crate rc-engine; plain release build, never `--features dev`)
#   rerac-extract[.exe]      the extractor (crate rc-extract)
#   assets/shaders/*.wgsl    the runtime's shaders (found next to the executable; rc-engine main.rs `asset_dir`)
#   rerac-manifest.json      the version manifest, version taken from [workspace.package] in Cargo.toml
#   README.txt
# Nothing from the disc is ever packaged: the folder is checked against that exact file list.
# The folder itself is a valid launcher version (the launcher's Development source can point at it).
#
# macOS and Windows (Git Bash): tested. Linux: written, UNTESTED.
set -euo pipefail

build=1
for a in "$@"; do
  case "$a" in
    --no-build) build=0 ;;
    -h|--help) sed -n '2,16p' "$0"; exit 0 ;;
    *) echo "unknown argument: $a" >&2; exit 2 ;;
  esac
done

repo="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$repo"

# The one workspace version: `version` under [workspace.package] (every crate inherits it).
version="$(awk '/^\[/ { in_pkg = ($0 == "[workspace.package]") } in_pkg && /^version = "/ { gsub(/^version = "|"$/, ""); print; exit }' Cargo.toml)"
[ -n "$version" ] || { echo "cannot read the version from [workspace.package] in Cargo.toml" >&2; exit 1; }

exe=""
case "$(uname -s)" in
  Darwin) os=macos ;;
  Linux) os=linux ;;                                  # UNTESTED
  MINGW*|MSYS*|CYGWIN*) os=windows; exe=".exe" ;;
  *) echo "unsupported OS: $(uname -s)" >&2; exit 1 ;;
esac
case "$(uname -m)" in
  arm64|aarch64) arch=arm64 ;;
  x86_64|amd64) arch=x86_64 ;;
  *) arch="$(uname -m)" ;;
esac

name="rerac-$version-$os-$arch"
target="${CARGO_TARGET_DIR:-$repo/target}/release"
dist="$repo/dist"
out="$dist/$name"

if [ "$build" = 1 ]; then
  # One cargo invocation for both binaries. No `--features dev`: that links Bevy dynamically.
  start=$(date +%s)
  cargo build --release --locked -p rc-engine -p rc-extract --bins
  echo "build: $(( $(date +%s) - start )) s"
fi
for b in rerac rerac-extract; do
  [ -x "$target/$b$exe" ] || { echo "missing $target/$b$exe (run without --no-build)" >&2; exit 1; }
done

rm -rf "$out" "$dist/$name.zip"
mkdir -p "$out/assets"
cp "$target/rerac$exe" "$target/rerac-extract$exe" "$out/"
cp -R crates/rc-engine/assets/. "$out/assets/"
find "$out/assets" -name '.*' -type f -delete

cat > "$out/rerac-manifest.json" <<EOF
{"schema":1,"name":"rerac","version":"$version","game":"rac1","runtime":"rerac$exe","extractor":"rerac-extract$exe","supported_discs":["SCUS_971.99"],"data_format":1}
EOF

cat > "$out/README.txt" <<EOF
ReRAC $version ($os-$arch)
A reimplementation of Ratchet & Clank (PS2). It contains no game data: it runs from data
extracted once from your own disc image (NTSC-U SCUS_971.99, .iso).

Normal use: add this folder as a version in the ReRAC launcher. The launcher runs the
extractor and starts the game.

By hand:
  ./rerac-extract identify --iso <your disc>.iso
  ./rerac-extract extract --iso <your disc>.iso --out <data folder>
  ./rerac --data-dir <data folder>

Files: rerac (the game), rerac-extract (the extractor), assets/ (shaders, must stay next
to rerac), rerac-manifest.json (the launcher's version manifest).
Exit codes of rerac: 2 bad arguments, 3 data folder missing or incomplete, 4 data made for
another version (re-extract).
EOF
if [ "$os" = windows ]; then
  sed -i 's#\./rerac#rerac#g; s/$/\r/' "$out/README.txt"
fi

# Allow-list: exactly the files above, and only .wgsl under assets/. Anything else fails the package.
unexpected="$(cd "$out" && find . -type f \
  ! -path "./rerac$exe" ! -path "./rerac-extract$exe" ! -path ./rerac-manifest.json ! -path ./README.txt \
  ! -path './assets/shaders/*.wgsl')"
if [ -n "$unexpected" ]; then
  echo "unexpected files in the package (allow-list in tools/package/package.sh):" >&2
  echo "$unexpected" >&2
  exit 1
fi

# The packaged runtime must answer the contract line with this version.
got="$("$out/rerac$exe" --version-json)"
want="{\"name\":\"rerac\",\"version\":\"$version\",\"game\":\"rac1\",\"data_format\":1}"
[ "$got" = "$want" ] || { echo "rerac --version-json: got $got, want $want" >&2; exit 1; }

case "$os" in
  macos) (cd "$dist" && ditto -c -k --keepParent "$name" "$name.zip") ;;
  # Windows' own bsdtar writes a standard zip ('/' separators); PowerShell 5.1's Compress-Archive writes '\'.
  windows) (cd "$dist" && "$(cygpath "$SYSTEMROOT")/System32/tar.exe" -a -c -f "$name.zip" "$name") ;;
  linux) (cd "$dist" && zip -qry "$name.zip" "$name") ;;  # UNTESTED; needs `zip`
esac

size=$(wc -c < "$dist/$name.zip" | tr -d ' ')
echo "packaged $out"
echo "zip $dist/$name.zip ($size bytes)"
