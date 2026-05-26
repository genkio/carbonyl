use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const HASH: &str = include_str!(concat!(env!("OUT_DIR"), "/hash.txt"));

struct Payload {
    name: &'static str,
    data: &'static [u8],
    exec: bool,
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const PAYLOAD: &[Payload] = &[
    Payload {
        name: "carbonyl",
        data: include_bytes!(concat!(env!("OUT_DIR"), "/carbonyl")),
        exec: true,
    },
    Payload {
        name: "libcarbonyl.dylib",
        data: include_bytes!(concat!(env!("OUT_DIR"), "/libcarbonyl.dylib")),
        exec: true,
    },
    Payload {
        name: "libEGL.dylib",
        data: include_bytes!(concat!(env!("OUT_DIR"), "/libEGL.dylib")),
        exec: true,
    },
    Payload {
        name: "libGLESv2.dylib",
        data: include_bytes!(concat!(env!("OUT_DIR"), "/libGLESv2.dylib")),
        exec: true,
    },
    Payload {
        name: "icudtl.dat",
        data: include_bytes!(concat!(env!("OUT_DIR"), "/icudtl.dat")),
        exec: false,
    },
    Payload {
        name: "v8_context_snapshot.arm64.bin",
        data: include_bytes!(concat!(env!("OUT_DIR"), "/v8_context_snapshot.arm64.bin")),
        exec: false,
    },
];

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
const PAYLOAD: &[Payload] = &[
    Payload {
        name: "carbonyl",
        data: include_bytes!(concat!(env!("OUT_DIR"), "/carbonyl")),
        exec: true,
    },
    Payload {
        name: "libcarbonyl.dylib",
        data: include_bytes!(concat!(env!("OUT_DIR"), "/libcarbonyl.dylib")),
        exec: true,
    },
    Payload {
        name: "libEGL.dylib",
        data: include_bytes!(concat!(env!("OUT_DIR"), "/libEGL.dylib")),
        exec: true,
    },
    Payload {
        name: "libGLESv2.dylib",
        data: include_bytes!(concat!(env!("OUT_DIR"), "/libGLESv2.dylib")),
        exec: true,
    },
    Payload {
        name: "icudtl.dat",
        data: include_bytes!(concat!(env!("OUT_DIR"), "/icudtl.dat")),
        exec: false,
    },
    Payload {
        name: "v8_context_snapshot.x86_64.bin",
        data: include_bytes!(concat!(env!("OUT_DIR"), "/v8_context_snapshot.x86_64.bin")),
        exec: false,
    },
];

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const PAYLOAD: &[Payload] = &[
    Payload {
        name: "carbonyl",
        data: include_bytes!(concat!(env!("OUT_DIR"), "/carbonyl")),
        exec: true,
    },
    Payload {
        name: "libcarbonyl.so",
        data: include_bytes!(concat!(env!("OUT_DIR"), "/libcarbonyl.so")),
        exec: true,
    },
    Payload {
        name: "libEGL.so",
        data: include_bytes!(concat!(env!("OUT_DIR"), "/libEGL.so")),
        exec: true,
    },
    Payload {
        name: "libGLESv2.so",
        data: include_bytes!(concat!(env!("OUT_DIR"), "/libGLESv2.so")),
        exec: true,
    },
    Payload {
        name: "libvk_swiftshader.so",
        data: include_bytes!(concat!(env!("OUT_DIR"), "/libvk_swiftshader.so")),
        exec: true,
    },
    Payload {
        name: "libvulkan.so.1",
        data: include_bytes!(concat!(env!("OUT_DIR"), "/libvulkan.so.1")),
        exec: true,
    },
    Payload {
        name: "vk_swiftshader_icd.json",
        data: include_bytes!(concat!(env!("OUT_DIR"), "/vk_swiftshader_icd.json")),
        exec: false,
    },
    Payload {
        name: "icudtl.dat",
        data: include_bytes!(concat!(env!("OUT_DIR"), "/icudtl.dat")),
        exec: false,
    },
    Payload {
        name: "v8_context_snapshot.bin",
        data: include_bytes!(concat!(env!("OUT_DIR"), "/v8_context_snapshot.bin")),
        exec: false,
    },
];

fn main() -> ! {
    let cache_dir = cache_dir();
    let ready = cache_dir.join(".ready");
    if !ready.exists() {
        if let Err(err) = extract(&cache_dir) {
            eprintln!("carbonyl-bundle: extract failed: {err}");
            std::process::exit(1);
        }
    }

    let bin = cache_dir.join("carbonyl");
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let err = Command::new(&bin).args(&args).exec();
    eprintln!("carbonyl-bundle: exec {} failed: {}", bin.display(), err);
    std::process::exit(1);
}

fn cache_dir() -> PathBuf {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(|| std::env::temp_dir());
    base.join("carbonyl").join(HASH.trim())
}

fn extract(dir: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    for entry in PAYLOAD {
        let target = dir.join(entry.name);
        let tmp = dir.join(format!(".{}.tmp", entry.name));
        fs::write(&tmp, entry.data)?;
        if entry.exec {
            fs::set_permissions(&tmp, fs::Permissions::from_mode(0o755))?;
        } else {
            fs::set_permissions(&tmp, fs::Permissions::from_mode(0o644))?;
        }
        fs::rename(&tmp, &target)?;
    }
    fs::write(dir.join(".ready"), HASH.trim())?;
    Ok(())
}
