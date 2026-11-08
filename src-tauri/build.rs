fn main() {
    // Manifesto do exe com <msix>: liga o Island.exe ao pacote esparso
    let windows = tauri_build::WindowsAttributes::new().app_manifest(include_str!("island.exe.manifest"));
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows))
        .expect("falha no build script");
}
