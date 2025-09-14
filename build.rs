use serde::Deserialize;
use std::env;
use std::fs::File;
use std::io::Write;
use std::process::Command;

// see https://haim.dev/posts/2020-09-10-linking-swift-code-into-rust-app/

/// Should match Package.swift
const MACOS_TARGET_VERSION: &str = "15.5";
const IOS_TARGET_VERSION: &str = "18.5";

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SwiftTargetInfo {
    triple: String,
    unversioned_triple: String,
    module_triple: String,
    #[serde(rename = "librariesRequireRPath")]
    libraries_require_rpath: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SwiftPaths {
    runtime_library_paths: Vec<String>,
    runtime_library_import_paths: Vec<String>,
    runtime_resource_path: String,
}

#[derive(Debug, Deserialize)]
struct SwiftTarget {
    target: SwiftTargetInfo,
    paths: SwiftPaths,
}

#[allow(dead_code)]
#[derive(Debug)]
struct Error(String);

impl<E: ToString> From<E> for Error {
    fn from(error: E) -> Self {
        Self(error.to_string())
    }
}

fn build_swift() -> Result<(), Error> {
    let pkg_name = env::var("CARGO_PKG_NAME")?;
    let profile = env::var("PROFILE")?;
    let mut target_arch = env::var("CARGO_CFG_TARGET_ARCH")?;
    if target_arch == "aarch64" {
        target_arch = "arm64".to_string();
    }
    let target_vendor = env::var("CARGO_CFG_TARGET_VENDOR")?;
    let target_os = env::var("CARGO_CFG_TARGET_OS")?;
    let target_abi = env::var("CARGO_CFG_TARGET_ABI")?;
    let target_os_versioned = match &target_os[..] {
        "macos" => format!("macosx{MACOS_TARGET_VERSION}"),
        "ios" => {
            if target_abi == "sim" {
                format!("ios{IOS_TARGET_VERSION}-simulator")
            } else {
                format!("ios{IOS_TARGET_VERSION}")
            }
        }
        _ => return Err("unimplemented".into()),
    };
    let sdk = match &target_os[..] {
        "macos" => "macosx",
        "ios" => "iphoneos",
        _ => return Err("unimplemented".into()),
    };
    let target = format!("{target_arch}-{target_vendor}-{target_os_versioned}");
    let mut swift = String::from_utf8(
        Command::new("xcrun")
            .args(&["-f", "-sdk", sdk, "swift"])
            .output()
            .unwrap()
            .stdout,
    )?;
    swift = swift.trim().to_string();
    eprintln!("target_os: {target_os}, sdk: {sdk}");
    eprintln!("swift: [{swift}]");

    let swift_target_info_str = Command::new(&swift[..])
        .args(&["-target", &target, "-print-target-info"])
        .output()?
        .stdout;
    let swift_target_info: SwiftTarget = serde_json::from_slice(&swift_target_info_str)?;
    if swift_target_info.target.libraries_require_rpath {
        panic!("Libraries require RPath! Change minimum MacOS value to fix.")
    }

    let bridge_files = vec!["src/main.rs"];
    swift_bridge_build::parse_bridges(bridge_files)
        .write_all_concatenated("generated", pkg_name.as_str());

    let mut header = File::create("generated/bridging_header.h")?;
    writeln!(header, "#ifndef BRIDGING_HEADER_H")?;
    writeln!(header, "#define BRIDGING_HEADER_H")?;
    writeln!(header, "#import \"SwiftBridgeCore.h\"")?;
    writeln!(header, "#import \"{pkg_name}/{pkg_name}.h\"")?;
    writeln!(header, "#endif")?;

    if !Command::new(&swift[..])
        .args(&[
            "build",
            "-c",
            &profile,
            "--triple",
            &target,
            "-Xswiftc",
            "-target",
            "-Xswiftc",
            &target,
            "-Xswiftc",
            "-import-objc-header",
            "-Xswiftc",
            "generated/bridging_header.h",
        ])
        // Leaving SDKROOT set breaks swift on the host when cross-compiling,
        // so that it can't evaluate Package.swift
        .env_remove("SDKROOT")
        .status()?
        .success()
    {
        panic!("Swift library compilation failed")
    }

    swift_target_info
        .paths
        .runtime_library_paths
        .iter()
        .for_each(|path| {
            println!("cargo:rustc-link-search=native={}", path);
        });
    println!(
        "cargo:rustc-link-search=native=./.build/{}/{}",
        swift_target_info.target.unversioned_triple, profile
    );
    println!("cargo:rustc-link-lib=static={pkg_name}");
    println!("cargo:rerun-if-changed=src/swift/*.swift");
    match &target_os[..] {
        "macos" => {
            println!(
                "cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET={}",
                MACOS_TARGET_VERSION
            );
        }
        "ios" => {
            println!(
                "cargo:rustc-env=IPHONEOS_DEPLOYMENT_TARGET={}",
                IOS_TARGET_VERSION
            );
        }
        _ => {}
    }
    Ok(())
}

fn main() -> Result<(), Error> {
    let target = env::var("CARGO_CFG_TARGET_OS")?;
    if target == "macos" || target == "ios" {
        build_swift()?;
    }
    Ok(())
}
