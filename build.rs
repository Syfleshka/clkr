//! Embeds the icon, manifest and version info into the Windows executable.

use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=assets/clkr.ico");
    println!("cargo:rerun-if-changed=assets/clkr.manifest");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let asset = |name: &str| Path::new(&dir).join("assets").join(name).display().to_string().replace('\\', "/");
    let version = std::env::var("CARGO_PKG_VERSION").unwrap();
    let mut parts: Vec<u16> = version.split('.').map(|p| p.parse().unwrap_or(0)).collect();
    parts.resize(4, 0);
    let numeric = parts.iter().map(u16::to_string).collect::<Vec<_>>().join(",");

    let rc = format!(
        r#"#pragma code_page(65001)
1 ICON "{icon}"
1 24 "{manifest}"
1 VERSIONINFO
FILEVERSION {numeric}
PRODUCTVERSION {numeric}
FILEOS 0x40004
FILETYPE 0x1
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904B0"
    BEGIN
      VALUE "FileDescription", "clkr auto clicker"
      VALUE "ProductName", "clkr"
      VALUE "FileVersion", "{version}"
      VALUE "ProductVersion", "{version}"
      VALUE "OriginalFilename", "clkr.exe"
      VALUE "LegalCopyright", "MIT License"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#,
        icon = asset("clkr.ico"),
        manifest = asset("clkr.manifest"),
    );
    let out = Path::new(&std::env::var("OUT_DIR").unwrap()).join("clkr.rc");
    std::fs::write(&out, rc).unwrap();
    embed_resource::compile(&out, embed_resource::NONE).manifest_optional().unwrap();
}
