#[cfg(windows)]
fn main() {
    use std::{env, fs, path::PathBuf, process::Command};

    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let icon = manifest.join("assets/branding/minicraft-rust.ico");
    println!("cargo:rerun-if-changed={}", icon.display());
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("output directory"));
    let rc_file = out.join("minicraft-rust.rc");
    let res_file = out.join("minicraft-rust.res");
    fs::write(
        &rc_file,
        format!(
            "1 ICON \"{}\"\n",
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
