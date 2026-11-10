// Embed the icon and version info into kz.exe when targeting Windows.
fn main() {
    println!("cargo:rerun-if-changed=assets/kazma.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/kazma.ico")
            .set("ProductName", "kazma")
            .set("FileDescription", "kazma - ASCII system cleaner & disk explorer")
            .set("CompanyName", "kazma contributors")
            .set("LegalCopyright", "MIT License")
            .set("OriginalFilename", "kz.exe")
            .set("InternalName", "kz");
        if let Err(e) = res.compile() {
            println!("cargo:warning=could not embed Windows resources: {e}");
        }
    }
}
