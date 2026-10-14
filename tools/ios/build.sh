#!/usr/bin/env bash
# Builds ReRAC for iOS (macOS with Xcode command line tools and rustup).
#
#   tools/ios/build.sh device      → dist/ios/ReRAC.app (iPhone) and dist/ios/ReRAC.ipa (unsigned, for Sideloadly/AltStore)
#   tools/ios/build.sh simulator   → dist/ios-sim/ReRAC.app (Xcode's iOS Simulator on an Apple Silicon Mac)
#
# The app is a plain bundle around the `rerac` executable (no Xcode project): Info.plist, the shaders in assets/
# (found next to the executable, rc-engine main.rs `asset_dir`) and the icon (docs/assets/rerac-logo.png). Env: BUNDLE_ID (default
# com.rerac.ios), PROFILE (default release), SIGN_IDENTITY (optional codesign identity for a device build).
set -euo pipefail

kind="${1:-device}"
root="$(cd "$(dirname "$0")/../.." && pwd)"
ws="$root"
bundle_id="${BUNDLE_ID:-com.rerac.ios}"
profile="${PROFILE:-release}"

case "$kind" in
  device)    target=aarch64-apple-ios;     platform=iPhoneOS;        out="$root/dist/ios" ;;
  simulator) target=aarch64-apple-ios-sim; platform=iPhoneSimulator; out="$root/dist/ios-sim" ;;
  *) echo "usage: $0 device|simulator" >&2; exit 2 ;;
esac

command -v xcrun >/dev/null || { echo "error: needs macOS with Xcode (xcrun not found)" >&2; exit 1; }
[ -x "$HOME/.cargo/bin/rustup" ] && export PATH="$HOME/.cargo/bin:$PATH"
command -v rustup >/dev/null || {
  echo "error: Rust is not installed: curl https://sh.rustup.rs -sSf | sh -s -- -y, then open a new terminal" >&2
  exit 1
}
rustup target add "$target" >/dev/null

export IPHONEOS_DEPLOYMENT_TARGET=16.0
echo "==> cargo build ($target, $profile)"
(cd "$ws" && cargo build --locked --profile "$profile" --target "$target" -p rc-engine --bin rerac)
bin="${CARGO_TARGET_DIR:-$ws/target}/$target/$profile/rerac"

version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ws/Cargo.toml" | head -1)"
app="$out/ReRAC.app"
rm -rf "$app"
mkdir -p "$app/assets"
cp "$bin" "$app/rerac"
cp -R "$ws/crates/rc-engine/assets/." "$app/assets/"
find "$app/assets" -name '.*' -type f -delete
sed -e "s/__BUNDLE_ID__/$bundle_id/" -e "s/__VERSION__/${version%%-*}/" -e "s/__BUILD__/${GITHUB_RUN_NUMBER:-1}/" \
    -e "s/__PLATFORM__/$platform/" "$root/tools/ios/Info.plist" > "$app/Info.plist"
plutil -lint "$app/Info.plist" >/dev/null
icon="$ws/docs/assets/rerac-logo.png"
sips -z 120 120 "$icon" --out "$app/AppIcon60x60@2x.png" >/dev/null
sips -z 180 180 "$icon" --out "$app/AppIcon60x60@3x.png" >/dev/null
printf 'APPL????' > "$app/PkgInfo"

if [ "$kind" = simulator ]; then
  # The simulator runs only signed code; an ad-hoc signature is enough.
  codesign --force --sign - --timestamp=none "$app"
elif [ -n "${SIGN_IDENTITY:-}" ]; then
  codesign --force --sign "$SIGN_IDENTITY" --timestamp=none "$app"
fi

if [ "$kind" = device ]; then
  rm -rf "$out/Payload" "$out/ReRAC.ipa"
  mkdir -p "$out/Payload"
  cp -R "$app" "$out/Payload/"
  (cd "$out" && zip -qry ReRAC.ipa Payload && rm -rf Payload)
  echo "==> $out/ReRAC.ipa"
fi
echo "==> $app"
