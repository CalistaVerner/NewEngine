use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    // Re-run staging if the plugin crate changes.
    println!("cargo:rerun-if-changed=../../crates/newengine-modules-input/Cargo.toml");
    println!("cargo:rerun-if-changed=../../crates/newengine-modules-input/src");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());

    // apps/editor -> apps -> workspace root
    let workspace_root = find_workspace_root();
    let target_dir = workspace_root.join("target").join(&profile);
    let modules_dir = target_dir.join("modules");
    let deps_dir = target_dir.join("deps");

    let plugin_crate_name = "input";

    let version = read_pkg_version(&workspace_root.join("crates/newengine-modules-input/Cargo.toml"))
        .unwrap_or_else(|| "0.0.0".to_string());

    let src_dll = find_built_dll(&deps_dir, plugin_crate_name)
        .or_else(|| find_built_dll(&target_dir, plugin_crate_name));

    let Some(src_dll) = src_dll else {
        println!(
            "cargo:warning=plugin staging skipped: '{}' dll not found (searched: '{}', '{}')",
            plugin_crate_name,
            deps_dir.display(),
            target_dir.display()
        );
        return;
    };

    if let Err(e) = fs::create_dir_all(&modules_dir) {
        println!(
            "cargo:warning=plugin staging failed: create_dir_all('{}') ({})",
            modules_dir.display(),
            e
        );
        return;
    }

    let dst_plain = modules_dir.join(format!("{plugin_crate_name}.dll"));
    let dst_versioned = modules_dir.join(format!("{plugin_crate_name}-{version}.dll"));

    if let Err(e) = copy_if_different(&src_dll, &dst_plain) {
        println!(
            "cargo:warning=plugin staging failed: {} -> {} ({})",
            src_dll.display(),
            dst_plain.display(),
            e
        );
        return;
    }

    // Optional: keep versioned copy (useful for archives/hot-reload).
    let _ = copy_if_different(&src_dll, &dst_versioned);

    println!(
        "cargo:warning=plugin staged: {} (and {})",
        dst_plain.display(),
        dst_versioned.display()
    );
}

fn find_workspace_root() -> PathBuf {
    let dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    dir.parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .unwrap_or(dir)
}

fn read_pkg_version(cargo_toml: &Path) -> Option<String> {
    let txt = fs::read_to_string(cargo_toml).ok()?;
    let mut in_pkg = false;

    for line in txt.lines() {
        let s = line.trim();

        if s.starts_with('[') {
            in_pkg = s == "[package]";
            continue;
        }
        if !in_pkg {
            continue;
        }

        if let Some(rest) = s.strip_prefix("version") {
            let rest = rest.trim_start();
            if let Some(rest) = rest.strip_prefix('=') {
                let v = rest.trim();
                let v = v.split('#').next().unwrap_or(v).trim();
                return Some(v.trim_matches('"').to_string());
            }
        }
    }

    None
}

fn find_built_dll(dir: &Path, crate_name: &str) -> Option<PathBuf> {
    let rd = fs::read_dir(dir).ok()?;
    for ent in rd.flatten() {
        let p = ent.path();

        if p.extension() != Some(OsStr::new("dll")) {
            continue;
        }

        let Some(stem) = p.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };

        // Accept both:
        // - input.dll
        // - input-<hash>.dll (rare, but possible)
        if stem == crate_name || stem.starts_with(&format!("{crate_name}-")) {
            return Some(p);
        }
    }
    None
}

fn copy_if_different(src: &Path, dst: &Path) -> std::io::Result<()> {
    let need_copy = match (fs::metadata(src), fs::metadata(dst)) {
        (Ok(ms), Ok(md)) => ms.len() != md.len(),
        (Ok(_), Err(_)) => true,
        _ => true,
    };

    if !need_copy {
        return Ok(());
    }

    if let Some(parent) = dst.parent() {
        let _ = fs::create_dir_all(parent);
    }

    fs::copy(src, dst)?;
    Ok(())
}