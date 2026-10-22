#[cfg(windows)]
fn main() {
    use std::{env, fs, path::PathBuf, process::Command};

    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let icon = manifest.join("assets/branding/minicraft-rust.ico");
    println!("cargo:rerun-if-changed={}", icon.display());
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("output directory"));
    let rc_file = out.join("minicraft-rust.rc");
    let res_file = out.join("minicraft-rust.res");
    let version = env::var("CARGO_PKG_VERSION").expect("Cargo application version");
    let components: Vec<u16> = version
        .split('.')
        .map(|part| part.parse().expect("numeric release version"))
        .collect();
    assert_eq!(
        components.len(),
        3,
        "release version must have three components"
    );
    let numeric_version = format!("{},{},{},0", components[0], components[1], components[2]);
    fs::write(
        &rc_file,
        format!(
            r#"1 ICON "{}"
1 VERSIONINFO
 FILEVERSION {numeric_version}
 PRODUCTVERSION {numeric_version}
 FILEFLAGSMASK 0x3fL
 FILEFLAGS 0
 FILEOS 0x40004L
 FILETYPE 0x1L
BEGIN
 BLOCK "StringFileInfo"
 BEGIN
  BLOCK "040904b0"
  BEGIN
   VALUE "FileDescription", "Minicraft-Rust"
   VALUE "FileVersion", "{version}"
   VALUE "InternalName", "minicraft-rust"
   VALUE "OriginalFilename", "minicraft-rust.exe"
   VALUE "ProductName", "Minicraft-Rust"
   VALUE "ProductVersion", "{version}"
   VALUE "LegalCopyright", "Original Minicraft: Markus Persson; Minicraft+ contributors; GPL-3.0-only"
  END
 END
 BLOCK "VarFileInfo"
 BEGIN
  VALUE "Translation", 0x0409, 1200
 END
END
"#,
            icon.display().to_string().replace('\\', "/")
        ),
    )
    .expect("write Windows icon resource script");

    let sdk_bin = PathBuf::from(
        env::var_os("ProgramFiles(x86)").expect("Windows SDK program files directory"),
    )
    .join("Windows Kits/10/bin");
    let mut candidates = fs::read_dir(&sdk_bin)
        .expect("Windows SDK is required to embed the EXE icon")
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("x64/rc.exe"))
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    candidates.sort();
    let rc = candidates
        .pop()
        .expect("Windows SDK rc.exe is required to embed the EXE icon");
    let status = Command::new(rc)
        .arg("/nologo")
        .arg(format!("/fo{}", res_file.display()))
        .arg(&rc_file)
        .status()
        .expect("run Windows resource compiler");
    assert!(status.success(), "Windows icon resource compilation failed");
    println!(
        "cargo:rustc-link-arg-bin=minicraft-rust={}",
        res_file.display()
    );
}

#[cfg(not(windows))]
fn main() {}
