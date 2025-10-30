use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dest_path)?;
        } else {
            fs::copy(entry.path(), &dest_path)?;
        }
    }
    Ok(())
}

fn main() {
    // Paths
    let piper_root = Path::new("external/piper1-gpl/libpiper")
        .canonicalize()
        .unwrap();
    let install_dir = piper_root.join("install");

    let status = Command::new("cmake")
        .args([
            "-Bbuild",
            "-DCMAKE_BUILD_TYPE=Release",
            &format!("-DCMAKE_INSTALL_PREFIX={}", install_dir.display()),
        ])
        .current_dir(&piper_root)
        .status()
        .expect("Failed to run cmake configure");
    assert!(status.success(), "CMake configure failed");

    let status = Command::new("cmake")
        .args(["--build", "build"])
        .current_dir(&piper_root)
        .status()
        .expect("Failed to run cmake build");
    assert!(status.success(), "CMake build failed");

    let status = Command::new("cmake")
        .args(["--install", "build"])
        .current_dir(&piper_root)
        .status()
        .expect("Failed to run cmake install");
    assert!(status.success(), "CMake install failed");

    let espeak_data_src = install_dir.join("espeak-ng-data");
    let espeak_data_dst = Path::new("espeak-ng-data");

    if espeak_data_dst.exists() {
        fs::remove_dir_all(espeak_data_dst).expect("Failed to remove old espeak-ng-data");
    }

    copy_dir_all(&espeak_data_src, espeak_data_dst).expect("Failed to copy espeak-ng-data");

    fs::remove_dir_all(&espeak_data_src).expect("Failed to remove original espeak-ng-data");

    let libs_dir = Path::new("libs");
    if !libs_dir.exists() {
        fs::create_dir(libs_dir).expect("Failed to create libs folder");
    }

    // Copy libpiper.so from install root
    let libpiper_src = install_dir.join("libpiper.so");
    let libpiper_dst = libs_dir.join("libpiper.so");
    fs::copy(&libpiper_src, &libpiper_dst).unwrap_or_else(|_| {
        panic!(
            "Failed to copy {} to {}",
            libpiper_src.display(),
            libpiper_dst.display()
        )
    });

    // Copy other .so files from install/lib
    let other_libs = [
        "libonnxruntime.so",
        "libonnxruntime.so.1",
        "libonnxruntime.so.1.22.0",
    ];

    for lib in other_libs.iter() {
        let src = install_dir.join("lib").join(lib);
        let dst = libs_dir.join(lib);
        fs::copy(&src, &dst)
            .unwrap_or_else(|_| panic!("Failed to copy {} to {}", src.display(), dst.display()));
    }

  
    println!("cargo:rustc-link-search=native=libs");
    println!("cargo:rustc-link-lib=dylib=piper");
    println!("cargo:rustc-link-lib=dylib=onnxruntime");

    
    println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");

    
    let out_dir = env::var("OUT_DIR").unwrap();
    let target_dir = Path::new(&out_dir)
        .ancestors()
        .nth(3)
        .unwrap();

    let all_libs = [
        "libpiper.so",
        "libonnxruntime.so",
        "libonnxruntime.so.1",
        "libonnxruntime.so.1.22.0",
    ];
    for lib in all_libs.iter() {
        let src = libs_dir.join(lib);
        let dst = target_dir.join(lib);
        fs::copy(&src, &dst)
            .unwrap_or_else(|_| panic!("Failed to copy {} to {}", src.display(), dst.display()));
    }
}
