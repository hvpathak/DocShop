//! Embeds the app icon and version details in harshpad.exe, so Explorer, the
//! taskbar and the file's Properties dialog show them.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=../../assets/harshpad.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../assets/harshpad.ico");
        res.set("ProductName", "HarshPad");
        res.set("FileDescription", "HarshPad");
        res.set("LegalCopyright", "MIT License, HarshPad contributors");
        res.compile()?;
    }
    Ok(())
}
