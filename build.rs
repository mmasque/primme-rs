use std::env;
use std::path::{Path, PathBuf};

fn main() {
    let libdir_path = PathBuf::from("primme")
        // Canonicalize the path as `rustc-link-search` requires an absolute
        // path.
        .canonicalize()
        .expect("cannot canonicalize path");

    // This is the path to the `c` headers file.
    let headers_path = libdir_path.join("include/primme.h");
    let headers_path_str = headers_path.to_str().expect("Path is not a valid string");

    // Path to library .a file
    let lib_out_path = libdir_path.join("lib");
    let lib_out_path_str = lib_out_path.to_str().expect("Path is not a valid string");

    println!("Lib out path string: {:?}", lib_out_path);

    // Execute 'make' commands
    println!("cargo:rerun-if-changed={}", libdir_path.display()); // Invalidate build if C source changes
    println!("Building C library from: {:?}", libdir_path);

    // --- Build the static library ('make lib') ---
    println!("Running 'make lib' for static library...");
    let status_make_lib = std::process::Command::new("make")
        .arg("lib") // Execute the 'lib' target
        .current_dir(&libdir_path)
        .status()
        .expect("Failed to execute 'make lib'");

    if !status_make_lib.success() {
        panic!("Failed to build static C library with 'make lib'");
    }

    // Tell cargo to look for shared libraries in the specified directory
    println!("cargo:rustc-link-search={}", lib_out_path_str);

    // Tell cargo to tell rustc to link our `primme` library. Cargo will
    // automatically know it must look for a `libhello.a` file.
    println!("cargo:rustc-link-lib=primme");

    #[cfg(target_os = "macos")]
    println!("cargo:rustc-link-lib=framework=Accelerate");
    // Generate bindings
    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header(headers_path_str)
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("bindings.rs");
    bindings
        .write_to_file(out_path)
        .expect("Couldn't write bindings!");
}
