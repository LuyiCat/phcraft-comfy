#!/bin/sh
# Cross-compiles libphotocraft_android.so and assembles the signed PhotoComfy APK — no Gradle,
# no Java: the manifest names the system android.app.NativeActivity, which loads the library
# directly (see android/AndroidManifest.xml).
#
#   apps/photocraft-android/android/build.sh [OUT_DIR]     # from the repo root; default: out/android/
#
# PROFILE selects the Cargo profile, release by default (debug: PROFILE=debug). The SDK lives
# in ANDROID_HOME or ~/Library/Android/sdk and needs cmdline-tools, one NDK, build-tools and a
# platforms;android-* package (see README › Build). The default signing key is generated on
# first run; official releases point PHOTOCOMFY_ANDROID_KEYSTORE at the fixed release keystore
# and carry its passwords through PHOTOCOMFY_ANDROID_KEYSTORE_PASS / PHOTOCOMFY_ANDROID_KEY_PASS.
set -eu

OUT_DIR="${1:-out/android}"
PROFILE="${PROFILE:-release}"
LIB="photocraft_android"
ABI="arm64-v8a"
TARGET="aarch64-linux-android"
API="26"       # minimum: Android 8.0
API_TARGET="29"

# Xcode-style minimal PATHs are not the problem here, but the same probe keeps the scripts
# interchangeable: cargo may live in ~/.cargo/bin, or be managed by Homebrew, rustup, mise/asdf.
find_cargo() {
    if command -v cargo >/dev/null 2>&1; then
        command -v cargo
        return 0
    fi
    for candidate in "$HOME/.cargo/bin" /opt/homebrew/bin /usr/local/bin "$HOME/.local/bin" "$HOME/.rustup/toolchains"/*/bin "$HOME/.cargo"/*/bin; do
        if [ -x "$candidate/cargo" ]; then
            echo "$candidate/cargo"
            return 0
        fi
    done
    return 1
}

CARGO="$(find_cargo)" || {
    echo "error: cargo not found. Install Rust with rustup (https://rustup.rs), or add its bin" \
        "directory to this script's search list (apps/photocraft-android/android/build.sh find_cargo)." >&2
    exit 1
}
export PATH="$(dirname "$CARGO"):$PATH"
command -v cargo-ndk >/dev/null 2>&1 || {
    echo "error: cargo-ndk not found (it wires the NDK toolchain for Rust). Install it with" \
        "'cargo install cargo-ndk --locked' and add the aarch64-linux-android target" \
        "with 'rustup target add aarch64-linux-android'." >&2
    exit 1
}

SDK="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
[ -d "$SDK" ] || {
    echo "error: Android SDK not found at $SDK. Set ANDROID_HOME, or install cmdline-tools" \
        "(README › Build): sdkmanager 'build-tools' 'platform-tools' 'platforms;android-36' ndk emulator." >&2
    exit 1
}
NDK="${ANDROID_NDK_HOME:-$(ls -d "$SDK/ndk"/* 2>/dev/null | sort | tail -1)}"
BT="$(ls -d "$SDK/build-tools"/* 2>/dev/null | sort | tail -1)"
PLATFORM="$(ls -d "$SDK/platforms"/android-* 2>/dev/null | sort | tail -1)"
[ -n "$NDK" ] && [ -d "$BT" ] && [ -d "$PLATFORM" ] || {
    echo "error: the SDK at $SDK lacks an NDK, build-tools or a platforms;android-* package" \
        "(README › Build lists the sdkmanager invocation)." >&2
    exit 1
}
export ANDROID_HOME="$SDK" ANDROID_NDK_HOME="$NDK"

REPO="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$REPO"

# Keep Android intermediates separate from the desktop and iOS target trees. A caller's
# CARGO_TARGET_DIR must not make the copy step pick up a stale library from another platform.
export CARGO_TARGET_DIR="$REPO/target/android"

# Embed the craft-fonts faces like a desktop build (docs/development.md › Fonts): Android, like
# iPadOS, gives a NativeActivity app no usable fontconfig, so without these the UI's Chinese and
# other script text has no glyphs to fall back to (readable system Noto paths are still tried
# first; see crates/text/src/cjk.rs). The ignored project checkout (`craft-fonts/`) is picked up
# first; the old sibling checkout remains a fallback. CRAFT_FONTS_DIR set outside wins.
if [ -z "${CRAFT_FONTS_DIR:-}" ]; then
    # In a linked worktree the root lives under target/, so the main checkout next to the .git
    # dir is probed too. Without one of these, a build from a worktree embeds no fonts and every
    # CJK glyph on the device is tofu.
    MAIN="$(git -C "$REPO" rev-parse --path-format=absolute --git-common-dir 2>/dev/null | xargs dirname 2>/dev/null)"
    for root in "$REPO" "$MAIN"; do
        [ -n "$root" ] || continue
        for candidate in "$root/craft-fonts" "$root/../craft-fonts"; do
            if [ -d "$candidate/fonts" ]; then
                CRAFT_FONTS_DIR="$(cd "$candidate" && pwd)"
                export CRAFT_FONTS_DIR
                break 2
            fi
        done
    done
fi

VERSION="$(sed -n '/^\[workspace\.package\]/,/^\[/s/^version = "\(.*\)"$/\1/p' Cargo.toml | head -1)"
[ -n "$VERSION" ] || VERSION="0.0.0"
# The store build number: raise by one for every release, never reuse or rewind it.
VERSION_CODE="${VERSION_CODE:-2}"

echo "building lib${LIB}.so for $TARGET ($PROFILE)…"
cargo ndk -t "$ABI" -P "$API" build --profile "$PROFILE" -p photocraft-android

SO="$CARGO_TARGET_DIR/$TARGET/$PROFILE/lib${LIB}.so"
[ -f "$SO" ] || {
    echo "error: cargo produced no library at $SO" >&2
    exit 1
}

echo "assembling the APK with aapt2 ($BT)…"
# Absolute: the .so is zipped into the APK from inside $WORK/pkg below, after a cd.
WORK="$REPO/target/android/apk"
rm -rf "$WORK"
mkdir -p "$WORK/pkg/lib/$ABI" "$OUT_DIR"
cp "$SO" "$WORK/pkg/lib/$ABI/"
"$BT/aapt2" compile --dir "$(dirname "$0")/res" -o "$WORK/resources.zip"
"$BT/aapt2" link \
    -I "$PLATFORM/android.jar" \
    -R "$WORK/resources.zip" \
    --manifest "$(dirname "$0")/AndroidManifest.xml" \
    --min-sdk-version "$API" \
    --target-sdk-version "$API_TARGET" \
    --version-code "$VERSION_CODE" \
    --version-name "$VERSION" \
    -o "$WORK/base.apk"
# extractNativeLibs="true" in the manifest, so the library may ride compressed in the zip.
cd "$WORK/pkg" && zip -qr "$WORK/base.apk" lib
cd "$REPO"

"$BT/zipalign" -f 4 "$WORK/base.apk" "$WORK/aligned.apk"

# The default path is a local development key generated on first run: Android has no
# provisioning-profile expiry — teammates install the APK and keep it. Releases must instead
# point PHOTOCOMFY_ANDROID_KEYSTORE at the fixed release keystore backed up outside the repo,
# or every update after a rebuilt identity forces an uninstall first.
KEYSTORE="${PHOTOCOMFY_ANDROID_KEYSTORE:-$REPO/target/android/debug.keystore}"
STORE_PASS="${PHOTOCOMFY_ANDROID_KEYSTORE_PASS:-android}"
KEY_PASS="${PHOTOCOMFY_ANDROID_KEY_PASS:-$STORE_PASS}"
if [ ! -f "$KEYSTORE" ]; then
    if [ "$KEYSTORE" = "$REPO/target/android/debug.keystore" ]; then
        mkdir -p "$(dirname "$KEYSTORE")"
        echo "generating a signing key at $KEYSTORE (storepass/keypass 'android')…"
        keytool -genkeypair -keystore "$KEYSTORE" -storepass "$STORE_PASS" -keypass "$KEY_PASS" \
            -alias photocomfy -keyalg RSA -keysize 2048 -validity 10000 \
            -dname "CN=PhotoComfy Team, O=PhotoComfy, C=US"
    else
        echo "error: signing keystore $KEYSTORE not found. Point PHOTOCOMFY_ANDROID_KEYSTORE" \
            "at the existing release keystore, or unset it to build with a local development key." >&2
        exit 1
    fi
fi

APK="$OUT_DIR/PhotoComfy.apk"
"$BT/apksigner" sign --ks "$KEYSTORE" --ks-pass "pass:$STORE_PASS" --key-pass "pass:$KEY_PASS" \
    --ks-key-alias photocomfy --out "$APK" "$WORK/aligned.apk"
"$BT/apksigner" verify "$APK"
echo "installed $(wc -c < "$APK" | tr -d ' ') bytes into $APK ($(basename "$NDK") + build-tools $(basename "$BT"))"
echo "note: only $APK is signed — the intermediates in $WORK (base.apk,"
echo "      aligned.apk) have no signature and will not install on any device."
