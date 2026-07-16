fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        // Clear PE metadata reduces generic "unknown unsigned binary" heuristics a bit.
        res.set(
            "FileDescription",
            "dnstt-kit — DNS toolkit CLI (educational)",
        );
        res.set("ProductName", "dnstt-kit");
        res.set("ProductVersion", env!("CARGO_PKG_VERSION"));
        res.set("FileVersion", env!("CARGO_PKG_VERSION"));
        res.set("CompanyName", "BTF Kabir");
        res.set(
            "LegalCopyright",
            "Copyright (c) 2026 BTF Kabir — Non-commercial LICENSE",
        );
        res.set("OriginalFilename", "dns-cli.exe");
        res.set("InternalName", "dns-cli");
        if let Err(e) = res.compile() {
            eprintln!("cargo:warning=winres failed: {e}");
        }
    }
}
