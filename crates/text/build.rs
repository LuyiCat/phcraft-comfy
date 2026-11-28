//! Optional craft-fonts build input (https://github.com/storytold/craft-fonts, recipe from its
//! `docs/integration.md`).
//!
//! With `CRAFT_FONTS_DIR=<craft-fonts checkout>`, every font in its `fonts/manifest.txt` is
//! embedded and exposed as `photocraft_text::CRAFT_FONTS`; unset, `CRAFT_FONTS` is empty and the
//! build is exactly as before (an empty value counts as unset). Use an absolute path: build
//! scripts run in the crate's directory, so a relative one resolves from `crates/text`. A bad
//! checkout is a warning, or an error with `CRAFT_FONTS_REQUIRED=1` (release builds).
//!
//! PhotoCraft difference from the recipe: the web build (wasm32) embeds nothing. The wasm must
//! stay under the 24 MiB gate in `packaging/web/package.sh` (#237; Cloudflare's per-file cap is
//! 25 MiB). Measured 2026-10-06 (`trunk build --release`): 24,190,076 bytes
//! without craft-fonts, 28,867,488 with only the UI face (BIZ UDPGothic Regular) embedded, over
//! the 25,165,824-byte gate. The web build loads fonts served next to the wasm instead, on
//! demand (`fonts/manifest.txt`, see `served.rs` and packaging/web/README.md › Fonts).
//!
//! Also deflates the bundled UI fonts (`assets/fonts/*.ttf`, `fonts::BUNDLED`) into
//! `OUT_DIR/fonts/<file>.deflate`: 0.73 MB of binary instead of 1.5 MB, for the same size gate.
//!
//! The PhotoComfy font pack (docs/photocomfy-fonts.md) is a second optional input in the same
//! manifest format: with `PHOTOCOMFY_FONTS_DIR=<pack dir>` its faces join `CRAFT_FONTS` after the
//! craft fonts. The iOS and Android build scripts set it (and `PHOTOCOMFY_FONTS_REQUIRED=1`, which
//! turns a missing or bad pack into an error); desktop and web builds leave it unset.
//!
//! A manifest row may end with a ninth field, `lang=name;lang=name`: the family's name in each of
//! those languages for font menus (`CraftFont::names`). The pack's script writes it for families
//! whose own `name` table has no such record; craft-fonts rows have eight fields.
use std::fmt::Write as _;
use std::path::PathBuf;

fn main() {
    let wasm = std::env::var("CARGO_CFG_TARGET_ARCH").is_ok_and(|a| a == "wasm32");
    let mut src = String::from("pub static CRAFT_FONTS: &[CraftFont] = &[\n");
    embed_manifest("CRAFT_FONTS_DIR", "CRAFT_FONTS_REQUIRED", "craft-fonts", wasm, &mut src);
    embed_manifest("PHOTOCOMFY_FONTS_DIR", "PHOTOCOMFY_FONTS_REQUIRED", "the PhotoComfy font pack", wasm, &mut src);
    src.push_str("];\n");
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap_or_default()).join("craft_fonts.rs");
    if let Err(e) = std::fs::write(&out, src) {
        println!("cargo::error=writing {}: {e}", out.display());
    }
    deflate_bundled_fonts();
}

/// Appends the `CraftFont { .. }` initialisers of the `fonts/manifest.txt` in the directory named
/// by `dir_var` (`what`, in messages), when it is set (and not a wasm32 build). A bad directory is
/// a warning, or an error with `required_var` set — which also makes the variable's absence an
/// error.
fn embed_manifest(dir_var: &str, required_var: &str, what: &str, wasm: bool, src: &mut String) {
    println!("cargo::rerun-if-env-changed={dir_var}");
    println!("cargo::rerun-if-env-changed={required_var}");
    let required = std::env::var_os(required_var).is_some();
    let Some(dir) = std::env::var_os(dir_var).filter(|d| !d.is_empty() && !wasm).map(PathBuf::from) else {
        if required {
            println!("cargo::error={required_var} is set but {dir_var} is missing");
        }
        return;
    };
    match craft_fonts(&dir) {
        Ok(entries) => src.push_str(&entries),
        Err(e) if required => {
            println!("cargo::error={dir_var}={}: {e}", dir.display());
        }
        Err(e) => println!("cargo::warning=building without {what}: {dir_var}={}: {e}", dir.display()),
    }
}

/// The `lang=name;lang=name` field of a manifest row, absent or empty in most, as the elements of
/// `CraftFont::names`.
fn display_names(field: Option<&str>, line: &str) -> Result<Vec<String>, String> {
    let Some(field) = field.map(str::trim).filter(|f| !f.is_empty()) else { return Ok(Vec::new()) };
    field
        .split(';')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|pair| {
            let (lang, name) = pair
                .split_once('=')
                .filter(|(l, n)| !l.trim().is_empty() && !n.trim().is_empty())
                .ok_or_else(|| format!("malformed display name `{pair}` in manifest line: {line}"))?;
            Ok(format!("({:?}, {:?})", lang.trim(), name.trim()))
        })
        .collect()
}

/// Deflate the bundled fonts into `OUT_DIR/fonts/` (`fonts::BUNDLED` includes them).
fn deflate_bundled_fonts() {
    use std::io::Write as _;
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/fonts");
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap_or_default()).join("fonts");
    let result = (|| -> std::io::Result<()> {
        std::fs::create_dir_all(&out)?;
        for name in ["Inter-Regular.ttf", "Inter-Medium.ttf", "Inter-SemiBold.ttf", "JetBrainsMono-Regular.ttf"] {
            let path = src.join(name);
            println!("cargo::rerun-if-changed={}", path.display());
            let mut z = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::best());
            z.write_all(&std::fs::read(&path)?)?;
            std::fs::write(out.join(format!("{name}.deflate")), z.finish()?)?;
        }
        Ok(())
    })();
    if let Err(e) = result {
        println!("cargo::error=deflating the bundled fonts in {}: {e}", src.display());
    }
}

/// One `CraftFont { .. }` initialiser per manifest line.
fn craft_fonts(dir: &std::path::Path) -> Result<String, String> {
    let manifest = dir.join("fonts/manifest.txt");
    println!("cargo::rerun-if-changed={}", manifest.display());
    let text = std::fs::read_to_string(&manifest).map_err(|e| format!("{}: {e}", manifest.display()))?;
    let mut out = String::new();
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
        let f: Vec<&str> = line.split(" | ").map(str::trim).collect();
        let [family, style, file, scripts, ..] = f.as_slice() else {
            return Err(format!("malformed manifest line: {line}"));
        };
        let names = display_names(f.get(8).copied(), line)?;
        let path = dir.join(file).canonicalize().map_err(|e| format!("{file}: {e}"))?;
        println!("cargo::rerun-if-changed={}", path.display());
        let scripts: Vec<String> = scripts.split(',').map(|s| format!("{:?}", s.trim())).collect();
        let _ = writeln!(
            out,
            "    CraftFont {{ family: {family:?}, style: {style:?}, scripts: &[{}], bytes: include_bytes!({:?}), names: &[{}] }},",
            scripts.join(", "),
            path.display().to_string(),
            names.join(", "),
        );
    }
    Ok(out)
}
