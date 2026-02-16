/*!
 *  build-configuration/build.rs
 *  wake-word-detector
 * 
 *  SPDX-License-Identifier: MIT OR Apache-2.0
 *
 *  Copyright (c) 2021–2024 The rp-rs Developers
 *  Copyright (c) 2021 rp-rs organization
 *  Copyright (c) 2025 Raspberry Pi Ltd.
 *
 *  Set up linker scripts
 */

use std::fs::{ File, read_to_string };
use std::io::Write;
use std::path::PathBuf;

use regex::Regex;

/*
 *  Important note.
 * 
 *  Compile-time macros like `include_bytes!` and `include_str!`
 *  expect a file path relative to the source file they appear in.
 *  In this case, that means relative to the `build-configuration`
 *  directory.
 *
 *  However, paths used at runtime like ones passed to
 *  `read_to_string` or `File::create` expect a file path relative
 *  to the project root, which is where our Cargo.toml is located.
 *
 *  Finally, paths used with `cargo` directives like
 *  `cargo:rerun-if-changed=` also expect a file path relative
 *  to the project root, which is where our Cargo.toml is located.
 */

fn main() {
    println!("cargo::rustc-check-cfg=cfg(rp2040)");
    println!("cargo::rustc-check-cfg=cfg(rp2350)");

    // Put the linker script somewhere the linker can find it.
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    println!("cargo:rustc-link-search={}", out.display());

    println!("cargo:rerun-if-changed=.pico-rs");
    let contents = read_to_string("../.pico-rs")
        .map(|s| s.trim().to_string().to_lowercase())
        .unwrap_or_else(|e| {
            eprintln!("Failed to read file: {}", e);
            String::new()
        });

    // The file `memory.x` is loaded by cortex-m-rt's `link.x` script, which
    // is what we specify in `.cargo/config.toml` for ARM builds.
    let target;
    if contents == "rp2040" {
        target = "thumbv6m-none-eabi";
        let memory_x = include_bytes!("memory-layout/rp2040.x");
        let mut f = File::create(out.join("memory.x")).unwrap();
        f.write_all(memory_x).unwrap();
        println!("cargo::rustc-cfg=rp2040");
        println!("cargo:rerun-if-changed=build-configuration/memory-layout/rp2040.x");
    } else {
        if contents.contains("riscv") {
            target = "riscv32imac-unknown-none-elf";
        } else {
            target = "thumbv8m.main-none-eabihf";
        }
        let memory_x = include_bytes!("memory-layout/rp2350.x");
        let mut f = File::create(out.join("memory.x")).unwrap();
        f.write_all(memory_x).unwrap();
        println!("cargo::rustc-cfg=rp2350");
        println!("cargo:rerun-if-changed=build-configuration/memory-layout/rp2350.x");

        // TFLite Micro integration
        // TODO: move this to a helper function???
        let tflite_micro_path = PathBuf::from("../tflite-micro");
        let downloads_path = tflite_micro_path
            .join("tensorflow/lite/micro/tools/make/downloads");

        cc::Build::new()
            .cpp(true)
            .compiler("arm-none-eabi-g++")
            .flag("-std=c++17")
            .cpp_link_stdlib(None)
            .flag("-Wno-unused-parameter")
            .flag("-march=armv8-m.main+fp+dsp")
            .flag("-mcpu=cortex-m33")
            .flag("-mfloat-abi=hard")
            .flag("-mfpu=fpv5-sp-d16")
            .flag("-mthumb")
            .flag("-fno-rtti")
            .flag("-fno-exceptions")
            .flag("-fno-threadsafe-statics")
            .flag("-fno-unwind-tables")
            .flag("-ffunction-sections")
            .flag("-fdata-sections")
            // Tell TF Lite to use the arena-based allocator instead of `malloc`.
            .define("TF_LITE_STATIC_MEMORY", None)
            .define("TF_LITE_MCU_DEBUG_LOG", None)
            .define("NDEBUG", None)
            .include(&tflite_micro_path)
            .include(downloads_path.join("flatbuffers/include"))
            .include(downloads_path.join("gemmlowp"))
            .include(downloads_path.join("ruy"))
            .file("tflite-wrapper/tflite_wrapper.cc")
            .compile("tflite_wrapper");

        let lib_path = tflite_micro_path
            .join("gen/cortex_m_generic_cortex-m33_release_with_logs_cmsis_nn_gcc/lib");
        println!("cargo:rustc-link-search=native={}", lib_path.display());
        println!("cargo:rustc-link-lib=static=tensorflow-microlite");

        // Link the ARM toolchain's C library (newlib) for libc/libm
        // functions that TFLite Micro depends on.
        let gcc_print_file = |name: &str| -> PathBuf {
            let output = std::process::Command::new("arm-none-eabi-gcc")
                .args([
                    "-mcpu=cortex-m33",
                    "-mfloat-abi=hard",
                    "-mfpu=fpv5-sp-d16",
                    "-mthumb",
                    &format!("-print-file-name={}", name),
                ])
                .output()
                .expect("Failed to run arm-none-eabi-gcc");
            PathBuf::from(String::from_utf8(output.stdout).unwrap().trim())
        };

        let libc_path = gcc_print_file("libc.a");
        println!("cargo:rustc-link-search=native={}", libc_path.parent().unwrap().display());

        let libgcc_path = gcc_print_file("libgcc.a");
        println!("cargo:rustc-link-search=native={}", libgcc_path.parent().unwrap().display());

        println!("cargo:rustc-link-arg=--defsym=end=__sheap");
        println!("cargo:rustc-link-lib=static=c");
        println!("cargo:rustc-link-lib=static=m");
        println!("cargo:rustc-link-lib=static=nosys");
        println!("cargo:rustc-link-lib=static=gcc");

        println!("cargo:rerun-if-changed=tflite-wrapper/tflite_wrapper.cc");
        println!("cargo:rerun-if-changed=tflite-wrapper/tflite_wrapper.h");
    }

    // The file `rp2350_riscv.x` is what we specify in `.cargo/config.toml` for
    // RISC-V builds
    let rp2350_riscv_x = include_bytes!("memory-layout/rp2350_riscv.x");
    let mut f = File::create(out.join("rp2350_riscv.x")).unwrap();
    f.write_all(rp2350_riscv_x).unwrap();
    println!("cargo:rerun-if-changed=build-configuration/memory-layout/rp2350_riscv.x");

    // Replace target in the file `.cargo/config.toml`
    // to match the target inferred from the `.pico-rs` file.
    let re = Regex::new(r"target = .*").unwrap();
    let config_toml = include_str!("../.cargo/config.toml");
    let result = re.replace(config_toml, format!("target = \"{}\"", target));
    let mut f = File::create(".cargo/config.toml").unwrap();
    f.write_all(result.as_bytes()).unwrap();

    println!("cargo:rerun-if-changed=build-configuration/build.rs");
}
