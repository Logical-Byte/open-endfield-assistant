fn main() {
    println!("cargo:rerun-if-changed=tauri.conf.json");
    println!("cargo:rerun-if-changed=../../resources/icons/icon.ico");
    let metadata: serde_json::Value =
        serde_json::from_str(include_str!("tauri.conf.json")).expect("产品元数据必须是有效 JSON");
    let version = metadata["version"].as_str().expect("缺少版本号");
    let product = metadata["productName"].as_str().expect("缺少产品名称");
    println!("cargo:rustc-env=OEA_VERSION={version}");
    println!("cargo:rustc-env=OEA_PRODUCT_NAME={product}");
    // Windows 本机构建嵌入图标和版本。交叉检查只验证 Rust 代码，无需运行资源编译器。
    if cfg!(windows) && cfg!(feature = "desktop") && !cfg!(feature = "legacy-tauri") {
        let mut parts = [0u64; 4];
        for (slot, part) in parts.iter_mut().zip(version.split('.')) {
            *slot = part.parse().unwrap_or_default();
        }
        let numeric_version = parts
            .into_iter()
            .fold(0, |result, part| (result << 16) | part);
        winresource::WindowsResource::new()
            .set_icon("../../resources/icons/icon.ico")
            .set("ProductName", product)
            .set("FileDescription", product)
            .set("FileVersion", version)
            .set("ProductVersion", version)
            .set_version_info(winresource::VersionInfo::FILEVERSION, numeric_version)
            .set_version_info(winresource::VersionInfo::PRODUCTVERSION, numeric_version)
            .compile()
            .expect("无法编译 Windows 图标和版本资源");
    }
    #[cfg(feature = "legacy-tauri")]
    tauri_build::build();
}
