# UiProbe: scripted taps on the simulator

A standalone XCUITest (`UiProbe.xcodeproj`, not part of PhotoComfy's Xcode project) that plays a JSON
list of steps against PhotoComfy on a simulator with real taps and typing, and writes a screenshot or
the accessibility tree wherever a step asks for one. It is how to check what only a touch shows —
the font menu's groups, a picked font, typing through the system keyboard — when nobody can click in
Simulator.app: the agent has no macOS accessibility permission, and XCUITest reaches the app from
inside the simulator. `agent.md` §1.4 requires the font menu to be checked this way.

```sh
# 1. A simulator app to drive (the Release no-AIGC build, as for an upload):
xcodebuild -project apps/photocraft-ios/ios/PhotoComfy.xcodeproj -scheme PhotoComfy -configuration Release \
  -sdk iphonesimulator -destination 'generic/platform=iOS Simulator' \
  -derivedDataPath apps/photocraft-ios/ios/.ios-build/derived-data/Release-sim \
  PHOTOCOMFY_DISABLE_AIGC=YES CODE_SIGNING_ALLOWED=NO build
# 2. Play a script (font-menu.json: open the font menu, pick 得意黑, type Chinese in it):
apps/photocraft-ios/uitest/run.sh <simulator-udid> apps/photocraft-ios/uitest/font-menu.json
```

`run.sh` reinstalls the app (so preferences and the last tool start clean), launches it with
`PHOTOCRAFT_OPEN_NEW=1` so a document is open, and drives it on that same simulator. Screenshots,
accessibility trees and `steps.log` (the steps that could not be done) land in `out/uitest/shots/`.
The script is read after the app launched, so the display size and the tap scale can be taken from
`frames.txt` and a first screenshot.

## Steps

| Step | Effect |
|---|---|
| `{"tap": [x, y]}` | tap at a point of the display (a screenshot's pixels over 2) |
| `{"label": "字体"}` | tap the first element whose accessibility label is exactly this (waits for it up to 10 s) |
| `{"waitLabel": "移动工具"}` | wait (up to 40 s) for such an element |
| `{"reveal": {"label": "横排文字工具", "swipe": [x1, y1, x2, y2]}}` | swipe again (up to 5 times) until the labelled element is on screen |
| `{"swipe": [x1, y1, x2, y2, hold]}` | press, drag slowly, hold for `hold` seconds at the end |
| `{"type": "中文字体"}` | type into whatever has keyboard focus |
| `{"key": "return"}` | press a key of the hardware keyboard |
| `{"sleep": 2}` | wait |
| `{"shot": "menu"}` | write `<step>-menu.png` |
| `{"tree": "menu"}` | write the accessibility tree to `<step>-menu.txt` |
| `{"launch": {"NAME": "value"}}` | relaunch with these environment variables (not in `run.sh`, which attaches) |

## What the steps must account for

- **Taps land in the app's logical space.** An iPad simulator may report it larger than its display
  (1042×1512 for 834×1210 points): a tap passed as a display point then lands at 0.8× of it. `run.sh`
  passes `PROBE_TAP_SCALE` (1.25) and `PROBE_SCREEN` (834x1210) to compensate; when a tap reaches
  exactly where it is aimed, set `PROBE_TAP_SCALE=1`.
- **The accessibility tree fills in late.** After a launch it is empty until the screen was touched,
  and a `label` step waits for its element rather than failing at once.
- **Popups are not in the tree.** The rows of an open font menu have no accessibility elements: tap
  them by display point, taken from a screenshot of the same state.
- **Layouts move.** Which options the bar shows follows the selected tool, and the bar scrolls: take
  positions from a screenshot of the same run's earlier step, or aim inside a wide target.
