#!/bin/sh
# Builds the Rust binary for PhotoComfy on iPadOS and copies it into the .app bundle.
#
# Called by the Xcode target's script phase as:
#   build.sh "$BUILT_PRODUCTS_DIR" "$PRODUCT_NAME" "$PLATFORM_NAME" "$ARCHS"
# and usable standalone with the same four arguments, e.g. from the repo root:
#   apps/photocraft-ios/ios/build.sh out PhotoComfy iphonesimulator arm64
#
# The Xcode environment names the SDK (PLATFORM_NAME) and architectures (ARCHS);
# PROFILE selects the Cargo profile, release by default (debug: PROFILE=debug).
set -eu

OUT_DIR="$1"
PRODUCT="$2"
PLATFORM="${3:-iphonesimulator}"
ARCHS="${4:-arm64}"

case "$PLATFORM" in
    iphoneos) TARGET="aarch64-apple-ios" ;;
    iphonesimulator)
        # The simulator runs the host architecture; PhotoComfy builds for arm64 (Apple silicon).
        case "$ARCHS" in
            *arm64*) TARGET="aarch64-apple-ios-sim" ;;
            *) TARGET="x86_64-apple-ios" ;;
        esac
        ;;
    *)
        echo "error: unsupported platform '$PLATFORM' (expected iphoneos or iphonesimulator)" >&2
        exit 1
        ;;
esac
PROFILE="${PROFILE:-release}"

# Xcode's script phases run with a minimal PATH — and can arrive without a usable HOME, which
# breaks both the probe below and rustup itself (the cargo shim resolves ~/.cargo and ~/.rustup
# through HOME). Restore HOME from the account database before anything else needs it.
if [ -z "${HOME:-}" ] || [ ! -d "$HOME" ]; then
    account_home="$(dscl . -read "/Users/$(id -un 2>/dev/null)" NFSHomeDirectory 2>/dev/null | awk '{ print $2 }')"
    if [ -n "$account_home" ] && [ -d "$account_home" ]; then
        HOME="$account_home"
        echo "note: HOME was unset or missing; using the account home $HOME"
    fi
fi
HOME="${HOME:-}"
export HOME

# cargo may be on PATH, in ~/.cargo/bin, or managed by Homebrew, rustup's own layout, mise or
# asdf — probe the usual homes instead of failing. /Users/*/.cargo/bin is the last resort for
# environments whose HOME points somewhere other than the real account home.
find_cargo() {
    if command -v cargo >/dev/null 2>&1; then
        command -v cargo
        return 0
    fi
    for candidate in "$HOME/.cargo/bin" /opt/homebrew/bin /usr/local/bin "$HOME/.local/bin" \
        "$HOME/.rustup/toolchains"/*/bin "$HOME/.cargo"/*/bin /Users/*/.cargo/bin; do
        if [ -x "$candidate/cargo" ]; then
            echo "$candidate/cargo"
            return 0
        fi
    done
    return 1
}

CARGO="$(find_cargo)" || {
    echo "error: cargo not found. Xcode's script phase runs with a minimal PATH; install Rust" \
        "with rustup (https://rustup.rs), or add its bin directory to this script's search" \
        "list (apps/photocraft-ios/ios/build.sh find_cargo)." >&2
    echo "diagnostics: HOME='$HOME' USER='${USER:-<unset>}' PATH='$PATH'" >&2
    for candidate in "$HOME/.cargo/bin" /opt/homebrew/bin /usr/local/bin /Users/*/.cargo/bin; do
        ls -ld "$candidate/cargo" >&2 2>/dev/null || echo "  no cargo in $candidate" >&2
    done
    exit 1
}
export PATH="$(dirname "$CARGO"):$PATH"
echo "cargo: $CARGO"

case "${PLATFORM_NAME-}" in
    "") ;; # standalone run: PLATFORM_NAME comes in as $3 below
    iphoneos | iphonesimulator) ;;
    *)
        echo "error: this target builds for iOS/iPadOS. Pick an iPhone or iPad simulator or device" \
            "as the run destination in Xcode's toolbar (not 'My Mac'); this platform was:" \
            "'${PLATFORM_NAME}'." >&2
        exit 1
        ;;
esac

REPO="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$REPO"

# Embed the craft-fonts faces like a desktop build (docs/development.md › Fonts): iPadOS, unlike
# macOS, lets no app read the system font files, so without these the UI's Chinese and other
# script text has no glyphs to fall back to. The ignored project checkout (`craft-fonts/`) is
# picked up first; the old sibling checkout remains a fallback. CRAFT_FONTS_DIR set outside wins.
if [ -z "${CRAFT_FONTS_DIR:-}" ]; then
    for candidate in "$REPO/craft-fonts" "$REPO/../craft-fonts"; do
        if [ -d "$candidate/fonts" ]; then
            CRAFT_FONTS_DIR="$(cd "$candidate" && pwd)"
            export CRAFT_FONTS_DIR
            break
        fi
    done
fi
if [ -z "${CRAFT_FONTS_DIR:-}" ]; then
    echo "error: Chinese and other non-Latin fonts are required. Check out craft-fonts at $REPO/craft-fonts or set CRAFT_FONTS_DIR." >&2
    exit 1
fi
export CRAFT_FONTS_REQUIRED=1

# The PhotoComfy font pack (docs/photocomfy-fonts.md): the Chinese faces the Type tool offers,
# since iPadOS lets no app read the system's. About 190 MB, fetched on the first build that needs
# it and kept in the ignored `photocomfy-fonts/` directory. PHOTOCOMFY_FONTS_DIR set outside wins
# and must already hold the pack (scripts/photocomfy-fonts.sh <directory> fills one in).
if [ -z "${PHOTOCOMFY_FONTS_DIR:-}" ]; then
    if [ ! -f "$REPO/photocomfy-fonts/fonts/manifest.txt" ]; then
        "$REPO/scripts/photocomfy-fonts.sh" "$REPO/photocomfy-fonts"
    fi
    PHOTOCOMFY_FONTS_DIR="$REPO/photocomfy-fonts"
    export PHOTOCOMFY_FONTS_DIR
fi
export PHOTOCOMFY_FONTS_REQUIRED=1

# The pinned winit/egui forks carry three fixes that live only as patch files
# (../patches/README.md): taps landing below their targets on iOS, the safe-area insets, and the
# zoom-rescale jitter. They were applied to the cargo git checkouts by hand, which any cargo
# clean/update silently undoes. Apply them here instead — every build self-heals — and clean the
# patched crates only when a patch actually changed something (--forward skips applied hunks).
apply_fork_patch() {
    patch_file="$1"
    # winit is a repository-root package, so its manifest directory is the checkout hash
    # (…/checkouts/winit-…/a517b38), not a directory literally named "winit". Filtering only
    # for paths ending in /winit or /egui silently skipped the touch-coordinate patch.
    for manifest in $(cargo metadata --format-version 1 2>/dev/null | tr ',' '\n' | sed -n 's/.*"manifest_path":"\([^"]*\)\/Cargo.toml".*/\1/p' | grep -E '(/checkouts/winit-[^/]+/[^/]+$)|(/egui$)'); do
        case "$(basename "$patch_file")" in
            *winit*)
                case "$manifest" in
                    */checkouts/winit-*) name=winit ;;
                    *) continue ;;
                esac
                ;;
            *)
                [ "$(basename "$manifest")" = "egui" ] || continue
                name=egui
                ;;
        esac
        # The patch roots at the fork's checkout. winit's manifest directory already IS that
        # root; egui's package directory is <checkout>/crates/egui, two levels below it.
        if [ "$name" = "winit" ]; then
            source_dir="$manifest"
        else
            source_dir=$(dirname "$manifest")
            [ "$(basename "$source_dir")" = "crates" ] && source_dir=$(dirname "$source_dir")
        fi
        # --forward skips already-applied hunks; a checkout the patch changed needs a clean.
        patch_out=$(patch -d "$source_dir" -p1 --forward < "$patch_file" 2>&1 || true)
        if echo "$patch_out" | grep -q "^patching file"; then
            echo "fork patch applied: $name <- $(basename "$patch_file") (in $source_dir)"
            cargo clean -p "$name" --profile "$PROFILE" --target "$TARGET" 2>/dev/null || cargo clean -p "$name" 2>/dev/null || true
        fi
        # Never mistake a skipped/failed patch for a repaired build. These sentinels are in the
        # patched sources themselves, so a fresh or changed checkout fails with a useful log.
        case "$name" in
            winit) patched_file=src/platform_impl/ios/window.rs; sentinel='drawable covers the whole window' ;;
            egui) patched_file=crates/egui/src/context.rs; sentinel='The true screen rect from the platform stands' ;;
        esac
        if ! grep -q "$sentinel" "$source_dir/$patched_file"; then
            echo "$patch_out" >&2
            echo "error: $(basename "$patch_file") was not applied to $source_dir" >&2
            exit 1
        fi
    done
}
for patch_file in "$(dirname "$0")/../patches/"*.patch; do
    [ -e "$patch_file" ] && apply_fork_patch "$patch_file"
done

echo "building photocraft-ios for $TARGET ($PROFILE)…"
# --offline keeps cargo from a network round-trip on every Xcode Run; the fingerprint check
# below makes repeats cheap, and an actually-needed dependency update is a deliberate act.
if [ "${PHOTOCOMFY_DISABLE_AIGC:-NO}" = "YES" ]; then
    echo "AIGC transport: disabled (explicit override)"
    cargo build --profile "$PROFILE" --target "$TARGET" --offline --no-default-features --features heif -p photocraft-ios
else
    echo "AIGC transport: enabled"
    cargo build --profile "$PROFILE" --target "$TARGET" --offline --features heif -p photocraft-ios
fi

BIN="target/$TARGET/$PROFILE/photocraft-ios"
if [ ! -f "$BIN" ]; then
    echo "error: cargo produced no binary at $BIN" >&2
    exit 1
fi

APP="$OUT_DIR/$PRODUCT.app"
mkdir -p "$APP"
cp "$BIN" "$APP/$PRODUCT"
# The licence texts of the fonts embedded in that binary (the SIL Open Font License requires them
# to travel with the font): Licences/ inside the bundle, which Xcode signs with the rest.
"$REPO/scripts/font-licences.sh" "$APP/Licences"
# Standalone runs (simctl install) need the bundle metadata too; under Xcode the Info.plist
# step provides it, so only fill in what's missing. The placeholders are Xcode build settings;
# standalone they get the same values the target's settings carry.
if [ ! -f "$APP/Info.plist" ]; then
    sed -e "s/\$(EXECUTABLE_NAME)/$PRODUCT/" \
        -e "s/\$(PRODUCT_BUNDLE_IDENTIFIER)/photoComfy/" \
        -e "s/\$(MARKETING_VERSION)/1.1.1/" \
        -e "s/\$(CURRENT_PROJECT_VERSION)/8/" \
        "$(dirname "$0")/PhotoComfy/Info.plist" > "$APP/Info.plist"
    printf 'APPL????' > "$APP/PkgInfo"
fi
echo "installed $(wc -c < "$APP/$PRODUCT" | tr -d ' ') bytes into $APP/$PRODUCT"
