use std::env;
use std::fs;
use std::path::PathBuf;

use sha2::{Digest, Sha256};

const FILES: &[&str] = &[
    "carbonyl",
    "libcarbonyl.dylib",
    "libEGL.dylib",
    "libGLESv2.dylib",
    "icudtl.dat",
    "v8_context_snapshot.x86_64.bin",
];

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let payload_dir = env::var("CARBONYL_PAYLOAD_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = env::var("HOME").expect("HOME");
            PathBuf::from(home).join("Downloads/carbonyl-0.0.3")
        });

    println!("cargo:rerun-if-env-changed=CARBONYL_PAYLOAD_DIR");

    let mut hasher = Sha256::new();
    for name in FILES {
        let src = payload_dir.join(name);
        let dst = out_dir.join(name);
        let bytes = fs::read(&src).unwrap_or_else(|e| {
            panic!(
                "missing payload file {}: {}\nset CARBONYL_PAYLOAD_DIR to the dir holding the 6 \
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
