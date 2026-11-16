fn main() {
    #[cfg(all(target_os = "windows", target_env = "msvc"))]
    add_spectre_link_search();
}

#[cfg(all(target_os = "windows", target_env = "msvc"))]
fn add_spectre_link_search() {
    use cc::windows_registry;
    use std::env;

    let Ok(target) = env::var("TARGET") else {
        return;
    };
    let Ok(arch) = env::var("CARGO_CFG_TARGET_ARCH") else {
        return;
    };
    let arch_dir = match arch.as_str() {
        "x86_64" => "x64",
        "x86" => "x86",
        "aarch64" | "arm64ec" => "arm64",
        "arm" => "arm32",
        _ => return,
    };

    if let Some(tool) = windows_registry::find_tool(&target, "cl.exe") {
        let spectre_libs = tool
            .path()
            .join(format!(r"..\..\..\..\lib\spectre\{arch_dir}"));
        if spectre_libs.exists() {
            println!("cargo:rustc-link-search=native={}", spectre_libs.display());
        }
    }
}
