//! Finds the `unicode-linebreak` source that Cargo fetched and makes its
//! private tables reachable from `main.rs`.
//!
//! The crate includes `src/shared.rs` and `src/tables.rs` into its root. They
//! are copied into `OUT_DIR` as one file with their items made `pub`, so that
//! `main.rs` reads the crate's own pair table rather than a transcription.

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let crate_dir = upstream_dir();
    let mut text = String::new();
    for name in ["shared.rs", "tables.rs"] {
        let path = crate_dir.join("src").join(name);
        text.push_str(
            &std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("could not read {}: {e}", path.display())),
        );
        text.push('\n');
        println!("cargo:rerun-if-changed={}", path.display());
    }
    std::fs::write(out.join("upstream.rs"), publicise(&text)).unwrap();

    // The algorithm, which `src/Break.mw` ports by hand, for `main.rs` to
    // fingerprint.
    let lib = crate_dir.join("src/lib.rs");
    std::fs::copy(&lib, out.join("lib.rs.txt")).unwrap();
    println!("cargo:rerun-if-changed={}", lib.display());

    println!("cargo:rustc-env=UPSTREAM_DIR={}", crate_dir.display());
    println!("cargo:rerun-if-changed=Cargo.toml");
}

/// Where Cargo put the `unicode-linebreak` this build depends on.
fn upstream_dir() -> PathBuf {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let manifest = Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("Cargo.toml");
    let out = Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--manifest-path"])
        .arg(&manifest)
        .output()
        .expect("could not run `cargo metadata`");
    assert!(out.status.success(), "`cargo metadata` failed");
    let meta: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let pkg = meta["packages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "unicode-linebreak")
        .expect("unicode-linebreak is not among the dependencies");
    Path::new(pkg["manifest_path"].as_str().unwrap())
        .parent()
        .unwrap()
        .to_path_buf()
}

/// `text` with its module-level items made `pub`, so that they can be named
/// from outside the module it is included into.
fn publicise(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 1024);
    for line in text.lines() {
        let rewritten = if line.starts_with("#![") {
            continue;
        } else if line.trim_start().starts_with("fn is_safe_pair") {
            format!("pub {}", line.trim_start())
        } else if line.starts_with("fn ")
            || line.starts_with("static ")
            || line.starts_with("const ")
        {
            format!("pub {line}")
        } else {
            line.to_string()
        };
        out.push_str(&rewritten);
        out.push('\n');
    }
    out
}
