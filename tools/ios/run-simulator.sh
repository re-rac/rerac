#!/usr/bin/env bash
# Builds ReRAC for the iOS Simulator, boots an iPhone 16 Pro Max, installs the app, copies your disc image into the
# app's Documents folder (where the first run extracts it) and launches it with its log in this terminal.
#
#   tools/ios/run-simulator.sh /path/to/Ratchet.iso            (first time: copies the ISO, the app extracts it)
#   tools/ios/run-simulator.sh                                  (later runs: the data is already extracted)
#
# Env: DEVICE (default "iPhone 16 Pro Max"), BUNDLE_ID (default com.rerac.ios), APP (a prebuilt ReRAC.app, e.g. the
# CI artifact, instead of building).
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
iso="${1:-}"
device="${DEVICE:-iPhone 16 Pro Max}"
bundle_id="${BUNDLE_ID:-com.rerac.ios}"
app="${APP:-}"

if [ -z "$app" ]; then
  "$root/tools/ios/build.sh" simulator
  app="$root/dist/ios-sim/ReRAC.app"
fi

# The newest available simulator with this name (create one on the newest iOS runtime if none exists).
udid="$(xcrun simctl list devices available -j | python3 -c '
import json, sys
name = sys.argv[1]
devs = [d for rt, ds in json.load(sys.stdin)["devices"].items() if "iOS" in rt for d in ds if d["name"] == name]
print(devs[-1]["udid"] if devs else "")' "$device")"
if [ -z "$udid" ]; then
  runtime="$(xcrun simctl list runtimes -j | python3 -c '
import json, sys
rts = [r for r in json.load(sys.stdin)["runtimes"] if r["platform"] == "iOS" and r["isAvailable"]]
print(rts[-1]["identifier"] if rts else "")')"
  [ -n "$runtime" ] || { echo "error: no iOS simulator runtime; install one in Xcode › Settings › Components" >&2; exit 1; }
  udid="$(xcrun simctl create "$device" "com.apple.CoreSimulator.SimDeviceType.$(echo "$device" | tr ' ' '-')" "$runtime")"
fi
echo "==> simulator $device ($udid)"
xcrun simctl boot "$udid" 2>/dev/null || true
open -a Simulator --args -CurrentDeviceUDID "$udid"
xcrun simctl bootstatus "$udid" -b >/dev/null

xcrun simctl terminate "$udid" "$bundle_id" 2>/dev/null || true
xcrun simctl install "$udid" "$app"
if [ -n "$iso" ]; then
  docs="$(xcrun simctl get_app_container "$udid" "$bundle_id" data)/Documents"
  mkdir -p "$docs"
  echo "==> copying $(basename "$iso") into the app's Documents"
  cp -c "$iso" "$docs/" 2>/dev/null || cp "$iso" "$docs/"
fi
echo "==> launching (first run: setup screen extracts the disc, then run this script again without the ISO)"
xcrun simctl launch --console-pty --terminate-running-process "$udid" "$bundle_id"
