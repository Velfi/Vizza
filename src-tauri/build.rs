fn main() {
    // Check for SIMD capabilities
    if cfg!(target_arch = "x86_64") {
        println!("cargo:rustc-cfg=target_arch_x86_64");

        // Check for AVX2 support
        if cfg!(target_feature = "avx2") {
            println!("cargo:rustc-cfg=target_feature_avx2");
        }

        // Check for FMA support
        if cfg!(target_feature = "fma") {
            println!("cargo:rustc-cfg=target_feature_fma");
        }
    }

    tauri_build::build()
}
