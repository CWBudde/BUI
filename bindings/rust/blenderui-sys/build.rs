//! Builds the C kit (kit + gpu + core) with the project's own CMake files and
//! links it statically. Set `BUI_SOURCE_DIR` to build from another checkout;
//! published crates carry the sources in `csrc/`.

use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    // BUI_SOURCE_DIR, else the repository around this crate, else the sources
    // vendored into the published crate (`csrc/`, see scripts/vendor_c.py)
    let root = std::env::var("BUI_SOURCE_DIR").map(PathBuf::from).unwrap_or_else(|_| {
        let repo = manifest.join("../../..");
        if repo.join("kit/include/BUI_kit.h").exists() { repo } else { manifest.join("csrc") }
    });
    let root = root.canonicalize().expect("BUI source tree not found (set BUI_SOURCE_DIR)");
    // Windows canonical paths are \\?\C:\...; compilers' /FI and -include can't take them.
    let root = PathBuf::from(root.to_string_lossy().trim_start_matches(r"\\?\").to_string());

    // Portable (stb_truetype) text on Windows too: the `portable-text`
    // feature or BUI_TEXT_STB=1. Other platforms always use it.
    let stb = std::env::var("CARGO_FEATURE_PORTABLE_TEXT").is_ok()
        || std::env::var("BUI_TEXT_STB").map(|v| v == "1").unwrap_or(false);
    let dst = cmake::Config::new(&root)
        .define("BUI_TEXT_STB", if stb { "ON" } else { "OFF" })
        .define("BUI_BUILD_TESTS", "OFF")
        .define("BUI_BUILD_EXAMPLES", "OFF")
        .define("BUI_BUILD_PYTHON_BINDINGS", "OFF")
        .define("BUI_BUILD_UI", "OFF")
        .define("BUI_BUILD_QT_BACKEND", "OFF")
        // Always optimized; on MSVC also the release C runtime Rust links.
        .profile("Release")
        .build_target("bui_kit")
        .build();

    let build = dst.join("build");
    for lib in ["kit", "gpu", "core"] {
        for sub in ["", "Release"] {
            println!("cargo:rustc-link-search=native={}", build.join(lib).join(sub).display());
        }
    }
    for lib in ["bui_kit", "bui_gpu", "bui_core"] {
        println!("cargo:rustc-link-lib=static={lib}");
    }
    let target = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target == "windows" {
        for lib in ["user32", "gdi32"] {
            println!("cargo:rustc-link-lib={lib}");
        }
    } else {
        println!("cargo:rustc-link-lib=m");
        println!("cargo:rustc-link-lib=pthread");
    }
    for dir in ["kit", "gpu", "core", "CMakeLists.txt"] {
        println!("cargo:rerun-if-changed={}", root.join(dir).display());
    }
    println!("cargo:rerun-if-env-changed=BUI_SOURCE_DIR");
    println!("cargo:rerun-if-env-changed=BUI_TEXT_STB");
    println!("cargo:include={}", root.join("kit/include").display());
}
