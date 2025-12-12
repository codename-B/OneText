use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn main() {
    // Always rerun if assets change so we recopy them beside the built binary.
    println!("cargo:rerun-if-changed=assets");

    // Copy assets into target/{profile}/assets for running the built binary directly.
    if let Ok(profile) = env::var("PROFILE") {
        let target_dir = env::var("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("target"));
        let dest = target_dir.join(&profile).join("assets");
        let src = Path::new("assets");
        if src.exists() {
            if let Err(err) = copy_dir_all(src, &dest) {
                println!("cargo:warning=Failed to copy assets: {}", err);
            }
        }
    }

    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        // Prefer the new app icon if the .ico is present; otherwise build without it.
        let ico_path = Path::new("assets/icon.ico");
        if ico_path.exists() {
            res.set_icon(ico_path.to_str().unwrap());
        } else if Path::new("assets/icon.svg").exists() {
            println!(
                "cargo:warning=assets/icon.ico missing; convert assets/icon.svg -> assets/icon.ico to bundle the app icon"
            );
        }
        res.compile().unwrap();
    }
}
