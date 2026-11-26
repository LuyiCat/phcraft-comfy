# photocraft-ios

PhotoComfy for iOS and iPadOS: the same `ui-egui` shell and engine as the desktop, run by eframe's
native runner on Metal. A plain Rust `[[bin]]` — winit's iOS backend calls `UIApplicationMain`
itself from `EventLoop::run_app`, so no Objective-C bootstrap and no exported symbols are
needed; the binary is the app bundle's `CFBundleExecutable`.

Touch input is egui input: winit's iOS backend delivers taps (and Apple Pencil force, as
`Force::Calibrated`) as egui `Touch`/pointer events, `input/stylus.rs` feeds the pressure, and
`touch_twins` de-duplicates double-delivered taps in `raw_input_hook`. Two fingers pan and pinch
the canvas and stand the tool down (Photoshop on iPad). The interface runs in Preferences ›
Interface › Touch Interface (on by default on iPadOS): magnified targets, always-visible scroll
bars, and the safe area respected at the screen edges. Services the OS doesn't provide an iPad
app (desktop file dialogs, the clipboard bridge, crash recovery) stay unset and the shell
degrades gracefully, as in the browser; preferences persist in the app container's
`Application Support/PhotoComfy` directory.

## Fonts

iPadOS, unlike macOS, lets no app read the system font files, so the UI's script fallback needs
the embedded craft-fonts faces: `ios/build.sh` picks up the project-root checkout
(`craft-fonts/`) automatically, exactly like a desktop build with `CRAFT_FONTS_DIR`. Mobile builds
require it and stop with an error if it is absent, rather than shipping missing glyphs. The system locale (for the
interface language and the CJK script choice) is read from `NSLocale` at startup.

The Type tool also offers the PhotoComfy font pack ([docs/photocomfy-fonts.md](../../docs/photocomfy-fonts.md)):
19 Chinese families — Noto Sans and Serif in the Simplified and Traditional sets, LXGW WenKai Lite, Smiley Sans,
Zhuque Fangsong, Yozai, the ZCOOL faces and the handwriting faces — that `ios/build.sh` embeds after the craft
fonts. The pack is not in git: the first build that needs it runs `scripts/photocomfy-fonts.sh` (about 190 MB of
downloads into the ignored `photocomfy-fonts/`, checked against `scripts/photocomfy-fonts.lock`), then sets
`PHOTOCOMFY_FONTS_DIR`. The licence texts of every embedded font ship inside the bundle as `Licences/`.

## Build and run

Use **Xcode only** for iOS builds, signing, installation, simulation and device runs. Open
`ios/PhotoComfy.xcodeproj`, select the PhotoComfy scheme and an iOS/iPadOS Device (or a connected
iPhone/iPad), then press **Run**. Do not invoke `ios/build.sh`, Cargo iOS targets, `xcodebuild`,
`xcrun`, `simctl`, standalone Simulator or another IDE directly. If the simulated device window
is not visible, open Xcode's DeviceHub.

The target's Xcode script phase internally runs `ios/build.sh` as part of the Xcode build and
Xcode signs the bundle. Both Xcode Debug and Release builds include the Image API integration
and HEIC/HEIF import. The Release archive therefore carries the same editing and generation
features. For a build that deliberately omits the Image API transport, set
`PHOTOCOMFY_DISABLE_AIGC=YES` in the Xcode build settings or scheme. For a real iPad with a free
Apple ID, set the target's Signing & Capabilities team to your Personal Team, connect the iPad,
and press Run. The free profile expires after 7 days — press Run again to re-sign.

## Sideload to a real iPad with a free Apple ID (no App Store review)

1. Connect the iPad and open `ios/PhotoComfy.xcodeproj` in Xcode.
2. Select the PhotoComfy target › Signing & Capabilities › check "Automatically manage
   signing" and pick your **Personal Team** (sign in with your free Apple ID under Xcode ›
   Settings › Accounts if prompted).
3. On the iPad, trust the developer: Settings › General › VPN & Device Management › your
   Apple ID › Trust (first run only).
4. Select your iPad as the run destination and press Run. Xcode builds through
   `ios/build.sh`, signs, and installs.
5. The free profile expires after **7 days**: run again from Xcode to re-sign (data is kept).
   A free account signs at most 3 apps at once. A paid Apple Developer Program membership
   ($99/year) lifts the expiry to a year and adds TestFlight, but is not needed for personal
   use.

## Opening and saving files

File ▸ Open presents the system document picker (`ios/PCDocumentPicker.m`, driven from safe
Rust through the C ABI in `src/picker.rs`; the picker reads the files — security scope and
all — inside its delegate and hands `(name, bytes)` to the shell, which answers the app's
pending dialog with the contents). File ▸ Save writes the document's real file in the app's
Documents folder (Files › On My iPad › PhotoComfy), so it saves back in place and appears in
Open Recent. File ▸ Save As first presents the system file picker for a destination; the
delegate returns that real path, the normal save flow writes it, and the picker grant is
kept for later saves. Folder pickers are not offered (as on the web). `src/picker.rs` holds
this crate's only unsafe: the pointer reads that copy the callback's bytes across the ABI.

Apple Pencil pressure/tilt arrives through winit's iOS touch events and the stylus feed, like
a pen on the desktop.

## Troubleshooting

- **Nothing appears when you Run (Xcode 26+)**: this Xcode has no Simulator.app — simulator
  windows live in **DeviceHub** (`Xcode ▸ Open Developer Tool ▸ DeviceHub`, or it opens on
  Run). If the device shows "Not running" there, double-click its row to open its window and
  check whether the app is already running.
- **Xcode says "Unable to launch … No such process" on Run**: the app is fine — the debugger
  attaching to this ~127 MB binary on a simulator times out and Xcode reports the launch as
  failed while the process comes up moments later (the launch trail in `Documents/launch.log`
  completes and the UI renders). The shared scheme already runs without a debugger attached
  (`selectedDebuggerIdentifier` empty in `PhotoComfy.xcscheme`), which keeps Run from reporting
  this at all. If you ever see it anyway, inspect the Device in Xcode's DeviceHub — the app is there.
- **UIScene adoption (iPadOS 26+/27)**: iOS 27 traps at launch for apps that don't adopt the
  UIScene lifecycle (`EXC_BREAKPOINT` in
  `UIApplicationEvaluateRuntimeIssueForNoSceneLifecycleAdoption`; iOS 26 only warns). The app
  adopts it without touching the winit fork or writing unsafe Rust: `build.rs` compiles an
  empty ObjC scene delegate (`ios/PCSceneDelegate.m`, force-loaded past the Apple linker's
  dead-strip of unreferenced members) into the binary, and Info.plist's
  `UIApplicationSceneManifest` names it (`UISceneDelegateClassName`). winit keeps driving the
  window through the classic path — its plain UIWindow attaches to the foreground scene the
  manifest creates. The delegate class name is the load-bearing part: a manifest without one
  leaves no foreground scene and every OS shows a black screen.
- **Relaunch rendered blank (first launch after install fine)**: fixed — once Info.plist
  adopts scenes, UIKit saves the scene session to `Library/Saved Application State` and
  restores it on the next launch, and the restored scene never attaches winit's freshly
  created window (egui's viewport stays its 10000x10000 default). The shell now drops its own
  saved scene state before the UI starts (`discard_saved_scene_state` in `src/main.rs`), so
  every launch takes the fresh-install path. Verified: four consecutive relaunches on the
  iOS 27.2 simulator render identically; iOS 27.0 and Xcode device runs likewise.
- **Blank screen for a while, then the app appears**: that is the cold start, not a fault — a
  freshly booted simulator's first launch takes ~13 s from launch to first frame (loading the
  ~127 MB binary and compiling the GPU pipelines); every later launch is ~1 s. The launch
  screen is dark to match the app, so it reads as loading rather than a white screen. Wait out
  the first launch after booting a simulator or a device restart.
- **Opens to a white screen on the device and quits at once**: usually the expired signing
  profile — a free profile lasts 7 days, and an expired app is killed the moment it launches.
  Run again from Xcode to re-sign; check Settings › General › VPN & Device Management.
- **Anything else**: the build writes a startup trail to `Documents/launch.log`
  (`launched / app created / first frame` — the missing step names where it stopped) and every
  Rust panic with a backtrace to `Documents/crash.log`, both readable over Finder file sharing
  or Files › On My iPad › PhotoComfy. Send them and the step names the failure.
