//! The Windows executable's icon (issue #14): `assets/icons/cadrs.ico` compiled into a resource
//! and linked into the binary, so Explorer, shortcuts and the taskbar show it: with MinGW's
//! `windres` for a GNU build (the release, cross-built from Linux), and with `winresource` (the
//! Windows SDK's resource compiler) for an MSVC build on Windows, which also gives the file its
//! version details. Other targets need nothing here (the window icon is set at run time by
//! `cadrs_app::app_icon`).

use std::path::PathBuf;
use std::process::Command;

fn main() {
    let ico = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/icons/cadrs.ico");
    println!("cargo:rerun-if-changed={}", ico.display());
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if os != "windows" {
        return;
    }
    if env == "msvc" {
        msvc_icon(&ico);
        return;
    }
    if env != "gnu" {
        return;
    }
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let rc = out.join("cadrs.rc");
    let ico = ico.canonicalize().expect("assets/icons/cadrs.ico");
    std::fs::write(
        &rc,
        format!(
            "1 ICON \"{}\"\n",
            ico.display().to_string().replace('\\', "/")
        ),
    )
    .expect("writing cadrs.rc");
    let obj = out.join("cadrs-icon.o");
    // The cross toolchain's windres, else a native one (MSYS2 on Windows).
    let windres = ["x86_64-w64-mingw32-windres", "windres"]
        .into_iter()
        .find(|w| {
            Command::new(w)
                .arg("--version")
                .output()
                .is_ok_and(|o| o.status.success())
        });
    let Some(windres) = windres else {
        println!("cargo:warning=windres not found: cadrs.exe is built without its icon");
        return;
    };
    let ok = Command::new(windres)
        .arg(&rc)
        .arg("-O")
        .arg("coff")
        .arg("-o")
        .arg(&obj)
        .status()
        .is_ok_and(|s| s.success());
    if !ok {
        println!("cargo:warning=windres failed: cadrs.exe is built without its icon");
        return;
    }
    println!("cargo:rustc-link-arg-bins={}", obj.display());
}

/// The icon and version details of an MSVC build, on a Windows host (an MSVC build cross-compiled
/// from elsewhere goes without).
fn msvc_icon(ico: &std::path::Path) {
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon(&ico.to_string_lossy());
        if let Err(e) = res.compile() {
            println!(
                "cargo:warning=the resource compiler failed ({e}): cadrs.exe is built without its icon"
            );
        }
    }
    #[cfg(not(windows))]
    {
        let _ = ico;
        println!("cargo:warning=an MSVC build off Windows: cadrs.exe is built without its icon");
    }
}
