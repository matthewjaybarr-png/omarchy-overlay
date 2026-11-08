fn main() {
    println!("cargo:rerun-if-changed=app.manifest");
    println!("cargo:rerun-if-changed=assets/OmarchyOverlay.ico");
    println!("cargo:rerun-if-env-changed=RC");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let root = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let path = root.join("app.manifest");
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        // Windows copies the program into memory when it starts from removable or
        // network media, so it can restore the desktop after the stick is pulled out.
        println!("cargo:rustc-link-arg=/SWAPRUN:CD");
        println!("cargo:rustc-link-arg=/SWAPRUN:NET");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", path.display());
        let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
        let version = std::env::var("CARGO_PKG_VERSION").unwrap();
        let numbers = format!("{},0", version.replace('.', ","));
        let icon = root
            .join("assets/OmarchyOverlay.ico")
            .to_string_lossy()
            .replace('\\', "/");
        let resource = format!(
            r#"
101 ICON "{icon}"
1 VERSIONINFO
FILEVERSION {numbers}
PRODUCTVERSION {numbers}
FILEOS 0x40004
FILETYPE 1
BEGIN
 BLOCK "StringFileInfo"
 BEGIN
  BLOCK "040904B0"
  BEGIN
   VALUE "FileDescription", "Omarchy Overlay\0"
   VALUE "FileVersion", "{version}\0"
   VALUE "InternalName", "OmarchyOverlay\0"
   VALUE "OriginalFilename", "OmarchyOverlay.exe\0"
   VALUE "ProductName", "Omarchy Overlay\0"
   VALUE "ProductVersion", "{version}\0"
   VALUE "Comments", "Personal portable desktop prototype\0"
  END
 END
 BLOCK "VarFileInfo"
 BEGIN
  VALUE "Translation", 0x0409, 1200
 END
END
"#
        );
        let source = out.join("branding.rc");
        let compiled = out.join("branding.res");
        std::fs::write(&source, resource).expect("write branding resources");
        let compiler = std::env::var("RC")
            .unwrap_or_else(|_| if cfg!(windows) { "rc.exe" } else { "llvm-rc" }.into());
        let status = std::process::Command::new(compiler)
            .arg("/fo")
            .arg(&compiled)
            .arg(&source)
            .status()
            .expect("run Windows resource compiler (RC, rc.exe or llvm-rc)");
        assert!(
            status.success(),
            "Windows branding resource compilation failed"
        );
        println!("cargo:rustc-link-arg={}", compiled.display());
    }
}
