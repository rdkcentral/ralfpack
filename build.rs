//
// If not stated otherwise in this file or this component's LICENSE file the
// following copyright and licenses apply:
//
// Copyright 2025 Comcast Cable Communications Management, LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//

use sha2::{Digest, Sha256};
use std::error::Error;
use std::ffi::OsString;
use std::io::Seek;
use std::path::PathBuf;
use std::{env, fs, io, process};

/// Simple helper to copy files from one directory to another.
/// This will create the destination directory if it does not exist.
///
fn copy_file_utf8(from: &PathBuf, to: &PathBuf, filename: OsString) -> Result<(), Box<dyn Error>> {
    let utf8_file_name = filename
        .clone()
        .into_string()
        .map_err(|_| format!("unable to convert file name {:?} to UTF-8", filename))?;

    fs::create_dir_all(&to).map_err(|err| format!("creating directory {}: {}", to.display(), err))?;

    let src = from.join(&utf8_file_name);
    let dst = to.join(&utf8_file_name);

    fs::copy(&src, &dst).map_err(|err| format!("copying {} to {}: {}", src.display(), dst.display(), err))?;

    Ok(())
}

fn copy_erofs_files(source_dir: &PathBuf) -> Result<(), Box<dyn Error>> {
    let dst = PathBuf::from(env::var_os("OUT_DIR").ok_or("missing OUT_DIR environment variable")?);

    // Copy the include files to the destination directory
    let src_include = source_dir.join("include");
    let dst_include = dst.join("include");
    copy_file_utf8(&src_include, &dst_include, OsString::from("erofs_fs.h"))?;

    let src_include = src_include.join("erofs");
    let dst_include = dst_include.join("erofs");
    for e in fs::read_dir(&src_include)? {
        let e = e?;
        copy_file_utf8(&src_include, &dst_include, e.file_name())?;
    }

    // Copy the static library to a top-level directory
    let src_lib_dir = dst.join("build").join("lib").join(".libs");
    let dst_lib_dir = dst.join("lib");
    copy_file_utf8(&src_lib_dir, &dst_lib_dir, OsString::from("liberofs.a"))?;

    Ok(())
}

/// Calculates the SHA256 hash of a reader stream.
fn sha256_sum(data: &mut impl io::Read) -> io::Result<String> {
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 4096];

    loop {
        let n = data.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

/// Downloads the erofs-utils source code package from the official repository.
fn download_erfos_utils(version: &str, hash: &str) -> Result<PathBuf, Box<dyn Error>> {
    let out_dir = env::var("OUT_DIR").unwrap();
    let unpacked_path = PathBuf::from(&out_dir).join(format!("erofs-utils-{version}"));
    let url = format!("https://github.com/erofs/erofs-utils/archive/refs/tags/v{version}.tar.gz");

    // If already downloaded and extracted, skip
    if unpacked_path.exists() {
        println!(
            "erofs-utils version {version} already downloaded and extracted to {}, skipping download",
            unpacked_path.display()
        );
        return Ok(unpacked_path);
    }

    // Download using reqwest
    println!("Downloading erofs-utils version {version} from {url}");
    let response = reqwest::blocking::get(url).expect("Failed to download");
    let mut content = io::Cursor::new(response.bytes().expect("Failed to read bytes"));

    // Validate hash (SHA256) of the downloaded file
    let actual_hash = sha256_sum(&mut content).expect("Failed to compute hash or downloaded erofs tarball");
    if actual_hash != hash {
        return Err(format!("Hash mismatch: expected {hash}, got {actual_hash}").into());
    }

    // Extract the tarball
    content.rewind().expect("Failed to rewind downloaded content");
    let decompressed = flate2::read::GzDecoder::new(&mut content);
    let mut archive = tar::Archive::new(decompressed);
    archive.unpack(&out_dir).expect("Failed to extract archive");

    println!("Extracted erofs-utils version {version} to {}", unpacked_path.display());

    Ok(unpacked_path)
}

fn build_erofs_utils(source_dir: &PathBuf) -> Result<(), Box<dyn Error>> {
    // liblz4 should come from the "lz4-sys" rust package
    let lz4_include_dir = std::env::var_os("DEP_LZ4_INCLUDE");
    let lz4_lib_dir = std::env::var_os("DEP_LZ4_ROOT");
    if lz4_include_dir.is_none() || lz4_lib_dir.is_none() {
        panic!("DEP_LZ4_INCLUDE or DEP_LZ4_ROOT is not set - make sure to include the lz4-sys package in your build");
    }
    let lz4_include_dir = lz4_include_dir.unwrap();
    let lz4_lib_dir = lz4_lib_dir.unwrap();

    // zlib should come from the "libz-sys" rust package
    let zlib_include_dir = std::env::var_os("DEP_Z_INCLUDE");
    let zlib_lib_dir = std::env::var_os("DEP_Z_ROOT");
    if zlib_include_dir.is_none() || zlib_lib_dir.is_none() {
        panic!("DEP_Z_INCLUDE or DEP_Z_ROOT is not set - make sure to include the libz-sys package in your build");
    }
    let zlib_include_dir = zlib_include_dir.unwrap();
    let zlib_lib_dir = zlib_lib_dir.unwrap();

    // libzstd should come from the "zstd-sys" rust package
    let zstd_include_dir = std::env::var_os("DEP_ZSTD_INCLUDE");
    let zstd_lib_dir = std::env::var_os("DEP_ZSTD_ROOT");
    if zstd_include_dir.is_none() || zstd_lib_dir.is_none() {
        panic!(
            "DEP_ZSTD_INCLUDE or DEP_ZSTD_ROOT is not set - make sure to include the zstd-sys package in your build"
        );
    }
    let zstd_include_dir = zstd_include_dir.unwrap();
    let zstd_lib_dir = zstd_lib_dir.unwrap();

    // Patch the configure script to not check liblz4 as it insists on using pkg-config which
    // does not work with the lz4-sys package
    let configure_path = source_dir.join("configure.ac");
    let mut configure_content = fs::read_to_string(&configure_path)?;

    // Disable the libzstd check for now as it also needs work to support static libs
    configure_content = configure_content.replace(
        r#"have_libzstd="no"
AS_IF([test "x$with_libzstd" != "xno"], ["#,
        r#"have_libzstd="yes"
AS_IF([test "x$with_libzstd" == "xfoo"], ["#,
    );

    // Disable the liblz4 check and just assume we have it
    configure_content = configure_content.replace(
        r#"AS_IF([test "x$enable_lz4" != "xno"], ["#,
        r#"have_lz4="yes"
have_lz4hc="yes"
AS_IF([test "x$enable_lz4" == "xfoo"], ["#,
    );

    // Disable zlib check as well, just assume we have it
    configure_content = configure_content.replace(
        r#"have_zlib="no"
AS_IF([test "x$with_zlib" != "xno"], ["#,
        r#"have_zlib="yes"
AS_IF([test "x$with_zlib" == "xfoo"], ["#,
    );

    fs::write(&configure_path, configure_content)?;

    // For osxcross builds we need to specify the host and target as the autotools build
    // system does not pick it up from the environment automatically.  We also need to remove
    // any references to `--target=` in the CFLAGS as these are not understood by the compiler.

    // Get the cflags from the environment
    let compiler = cc::Build::new().get_compiler();
    let mut cflags = compiler.cflags_env().to_str().unwrap().to_string();
    cflags = cflags.replace("--target=x86_64-apple-macosx", "");
    cflags = cflags.replace("--target=arm64-apple-macosx", "");

    // Also add the include paths for zlib, lz4 and zstd to the standard cflags
    cflags += format!(" -I{} ", lz4_include_dir.to_str().unwrap()).as_str();
    cflags += format!(" -I{} ", zlib_include_dir.to_str().unwrap()).as_str();
    cflags += format!(" -I{} ", zstd_include_dir.to_str().unwrap()).as_str();

    // We supply the PKG_CONFIG=TRUE environment variable to the autotools build to skip
    // any pkg-config checks as we are manually specifying the include and library paths

    // Build the project in the path `thirdparty/erofs-utils` and installs it in `$OUT_DIR`
    let mut erofs_cfg = autotools::Config::new(source_dir);
    erofs_cfg
        .env("PKG_CONFIG", "true")
        .env("liblz4_LIBS", format!("{}/liblz4.a", lz4_lib_dir.to_str().unwrap()))
        .env("liblz4_CFLAGS", format!("-I {}", lz4_include_dir.to_str().unwrap()))
        .env("zlib_LIBS", format!("{}/lib/libz.a", zlib_lib_dir.to_str().unwrap()))
        .env("zlib_CFLAGS", format!("-I {}", zlib_include_dir.to_str().unwrap()))
        .env("libzstd_CFLAGS", format!("-I {}", zstd_include_dir.to_str().unwrap()))
        .env("libzstd_LIBS", format!("{}/libzstd.a", zstd_lib_dir.to_str().unwrap()))
        .env("CFLAGS", cflags)
        .reconf("-ivf")
        .disable("shared", None)
        .enable("static", None)
        .enable("lz4", None)
        .with("zlib", None)
        .with("libzstd", None)
        .without("xxhash", None)
        .disable("lzma", None)
        .without("uuid", None);

    // Check if the AUTOTOOLS_HOST environment variable is set and if so override the default --host
    // and --target options passed to the configure script.  This is needed for osxcross builds.
    if let Ok(host) = env::var("AUTOTOOLS_HOST") {
        erofs_cfg.config_option("host", Some(host.as_str()));
    }
    if let Ok(target) = env::var("AUTOTOOLS_TARGET") {
        erofs_cfg.config_option("target", Some(target.as_str()));
    }

    // Do the configure and build steps
    let erofs_utils = erofs_cfg.build();

    // Copy the erofs-utils files to the OUT_DIR
    copy_erofs_files(source_dir)?;

    // Add the erofs-utils include directory to the include path
    println!("cargo:include={}", erofs_utils.join("include").display());
    println!("cargo:rustc-link-search=native={}", erofs_utils.join("lib").display());
    println!("cargo:rustc-link-lib=static=erofs");

    // Require the lz4 and zlib libraries
    println!("cargo:rustc-link-lib=static=z");
    println!("cargo:rustc-link-lib=static=zstd");
    println!("cargo:rustc-link-lib=static=lz4");

    Ok(())
}

fn create_erofs_utils_bindings() -> Result<(), Box<dyn Error>> {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    // The path to the EROFS utils headers
    let include_path = out_path.join("include");

    // The bindgen::Builder is the main entry point to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // Create the bindings file for the "static inline" code in the headers
        .wrap_static_fns(true)
        .wrap_static_fns_path(out_path.join("erofs_static_wrapper").display().to_string())
        // The input header we would like to generate bindings for.
        .header("src/erofs/wrapper.h")
        // Tell bindgen where to find the erofs-utils headers.
        .clang_arg(format!("-I{}", include_path.display()))
        // Tell cargo to invalidate the built crate whenever any of the included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    // Build the C code that has the static inline wrapper functions
    cc::Build::new()
        .include(".")
        .include(&include_path)
        .flags([
            "-Wno-unused-parameter",
            "-Wno-visibility",
            "-Wno-ignored-qualifiers",
            "-Wno-sign-compare",
        ])
        .file(out_path.join("erofs_static_wrapper.c"))
        .compile("erofs_static_wrapper");

    Ok(())
}

fn main() {
    // Write build-time information to the OUT_DIR/built.rs file
    built::write_built_file().expect("Failed to acquire build-time information");

    // Re-run the build script if any of the erofs source files change
    println!("cargo:rerun-if-changed=src/erofs/");

    // Download and extract the erofs-utils source code
    let download_result = download_erfos_utils(
        "1.8.10",
        "05eb4edebe11decce6ecb34e98d2f80c8cd283c2f2967d8ba7efd58418570514",
    );
    if let Err(e) = download_result {
        eprintln!("{}", e);
        process::exit(1);
    }
    let erofs_utils_path = download_result.unwrap();

    // Build the erofs-utils static library
    let build_result = build_erofs_utils(&erofs_utils_path);
    if let Err(e) = build_result {
        eprintln!("{}", e);
        process::exit(1);
    }

    // Build the C++ code wrapping the erofs-utils library
    let binding_result = create_erofs_utils_bindings();
    if let Err(e) = binding_result {
        eprintln!("{}", e);
        process::exit(1);
    }
}
