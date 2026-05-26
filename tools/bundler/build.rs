use std::env;
use std::fs;
use std::path::PathBuf;

use sha2::{Digest, Sha256};

const MAC_ARM64_FILES: &[&str] = &[
    "carbonyl",
    "libcarbonyl.dylib",
    "libEGL.dylib",
    "libGLESv2.dylib",
    "icudtl.dat",
    "v8_context_snapshot.arm64.bin",
];

const MAC_X86_64_FILES: &[&str] = &[
    "carbonyl",
    "libcarbonyl.dylib",
    "libEGL.dylib",
    "libGLESv2.dylib",
    "icudtl.dat",
    "v8_context_snapshot.x86_64.bin",
];

const LINUX_X86_64_FILES: &[&str] = &[
    "carbonyl",
    "libcarbonyl.so",
    "libEGL.so",
    "libGLESv2.so",
    "libvk_swiftshader.so",
    "libvulkan.so.1",
    "vk_swiftshader_icd.json",
    "icudtl.dat",
    "v8_context_snapshot.bin",
];

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").expect("CARGO_CFG_TARGET_OS");
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").expect("CARGO_CFG_TARGET_ARCH");
    let files: &[&str] = match (target_os.as_str(), target_arch.as_str()) {
        ("macos", "aarch64") => MAC_ARM64_FILES,
        ("macos", "x86_64") => MAC_X86_64_FILES,
        ("linux", "x86_64") => LINUX_X86_64_FILES,
        (os, arch) => panic!("unsupported target {os}-{arch}"),
    };

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let payload_dir = env::var("CARBONYL_PAYLOAD_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = env::var("HOME").expect("HOME");
            PathBuf::from(home).join("Downloads/carbonyl-0.0.3")
        });

    println!("cargo:rerun-if-env-changed=CARBONYL_PAYLOAD_DIR");

    let mut hasher = Sha256::new();
    for name in files {
        let src = payload_dir.join(name);
        let dst = out_dir.join(name);
        let bytes = fs::read(&src).unwrap_or_else(|e| {
            panic!(
                "missing payload file {}: {}\nset CARBONYL_PAYLOAD_DIR to the dir holding the \
                 carbonyl runtime files",
                src.display(),
                e
            )
        });
        hasher.update(name.as_bytes());
        hasher.update(&bytes);
        fs::write(&dst, &bytes).expect("write payload");
        println!("cargo:rerun-if-changed={}", src.display());
    }

    let hash = format!("{:x}", hasher.finalize());
    fs::write(out_dir.join("hash.txt"), &hash[..16]).expect("write hash");
}
