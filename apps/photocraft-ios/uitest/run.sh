#!/bin/zsh
# Plays a JSON list of steps (README.md) against PhotoComfy on a simulator with real taps.
#
#   apps/photocraft-ios/uitest/run.sh <simulator-udid> <steps.json> [PhotoComfy.app]
#
# The app is reinstalled from PhotoComfy.app (default: the Release-simulator product that
#   xcodebuild -sdk iphonesimulator -derivedDataPath ios/.ios-build/derived-data/Release-sim …
# leaves under Build/Products/Release-iphonesimulator), launched with PHOTOCOMFY_OPEN_NEW=1 so a
# document is open, then driven on the same simulator (no clone), so the launch environment counts.
# Output: out/uitest/shots (screenshots, accessibility trees, steps.log).
#
# Environment: PROBE_SCREEN (display size in points, default 834x1210), PROBE_TAP_SCALE (default
# 1.25: taps land at 0.8x of a display point on the iPad Pro 11-inch iOS 27 simulator; 1 when a tap
# reaches exactly where it is aimed, as frames.txt and a screenshot tell).
set -eu

REPO="$(cd "$(dirname "$0")/../../.." && pwd)"
UITEST="$REPO/apps/photocraft-ios/uitest"
UDID="${1:?simulator udid}"
SCRIPT="${2:?steps json}"
APP="${3:-$REPO/apps/photocraft-ios/ios/.ios-build/derived-data/Release-sim/Build/Products/Release-iphonesimulator/PhotoComfy.app}"
OUT="${PROBE_OUT:-$REPO/out/uitest/shots}"
DERIVED="$REPO/apps/photocraft-ios/ios/.ios-build/uitest-derived"

[ -d "$APP" ] || { echo "no app at $APP" >&2; exit 1; }
[ -f "$SCRIPT" ] || { echo "no script at $SCRIPT" >&2; exit 1; }

xcrun simctl boot "$UDID" >/dev/null 2>&1 || true
xcrun simctl bootstatus "$UDID" -b >/dev/null 2>&1
xcrun simctl terminate "$UDID" photoComfy >/dev/null 2>&1 || true
xcrun simctl uninstall "$UDID" photoComfy >/dev/null 2>&1 || true
xcrun simctl install "$UDID" "$APP"

xcodebuild build-for-testing -project "$UITEST/UiProbe.xcodeproj" -scheme UiProbeTests \
  -destination "platform=iOS Simulator,id=$UDID" -derivedDataPath "$DERIVED" CODE_SIGNING_ALLOWED=NO > "$REPO/out/uitest/build.log" 2>&1 \
  || { tail -20 "$REPO/out/uitest/build.log" >&2; exit 1; }

SIMCTL_CHILD_PHOTOCRAFT_OPEN_NEW=1 xcrun simctl launch "$UDID" photoComfy >/dev/null
sleep 4
mkdir -p "$OUT"
TEST_RUNNER_PROBE_ATTACH=1 TEST_RUNNER_PROBE_SCRIPT="$(cd "$(dirname "$SCRIPT")" && pwd)/$(basename "$SCRIPT")" \
TEST_RUNNER_PROBE_OUT="$OUT" TEST_RUNNER_PROBE_LAUNCH_ENV="" \
xcodebuild test-without-building -project "$UITEST/UiProbe.xcodeproj" -scheme UiProbeTests \
  -destination "platform=iOS Simulator,id=$UDID" -derivedDataPath "$DERIVED" -parallel-testing-enabled NO \
  CODE_SIGNING_ALLOWED=NO > "$REPO/out/uitest/run.log" 2>&1 \
  || { tail -20 "$REPO/out/uitest/run.log" >&2; exit 1; }
grep -E "Test Suite 'All tests'" "$REPO/out/uitest/run.log" | tail -1
echo "output: $OUT"
