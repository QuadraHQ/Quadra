use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const VR_SOURCES: [&str; 3] = ["vr_device.c", "vr_xr.c", "rosemary_glue.c"];
const VR_LIBRARY: &str = "libQuadraVR.dylib";
const MACOS_TARGET: &str = "x86_64-apple-macos11.0";
const LINKER_ENV: &str = "QUADRA_VR_LD64_LLD";
const COMPILER_ENV: &str = "QUADRA_VR_CLANG";
const CHECK_AGAIN_NEXT_BUILD: &str = "vr-toolchain-missing";
const MACH_O_LINKER: &str = "ld64.lld";
const RUST_LINKER: &str = "rust-lld";

fn main() {
    println!("cargo:rerun-if-changed=vr");
    println!("cargo:rerun-if-env-changed={LINKER_ENV}");
    println!("cargo:rerun-if-env-changed={COMPILER_ENV}");

    let out_directory = PathBuf::from(env::var("OUT_DIR").expect("cargo sets OUT_DIR"));
    let library = out_directory.join(VR_LIBRARY);

    if let Err(reason) = build_vr_library(&out_directory, &library) {
        println!("cargo:warning=VR is unavailable in this build: {reason}");
        println!(
            "cargo:rerun-if-changed={}",
            out_directory.join(CHECK_AGAIN_NEXT_BUILD).display()
        );
        fs::write(&library, []).expect("OUT_DIR is writable");
    }
}

fn build_vr_library(out_directory: &Path, library: &Path) -> Result<(), String> {
    let compiler = env::var(COMPILER_ENV).unwrap_or_else(|_| "clang".to_owned());
    let mut linker = linker_command(&compiler)
        .ok_or("neither ld64.lld (from lld) nor Rust's rust-lld was found")?;
    let sources = Path::new("vr");

    let mut objects = Vec::new();
    for source in VR_SOURCES {
        let object = out_directory.join(source).with_extension("o");
        run(Command::new(&compiler)
            .args(["-target", MACOS_TARGET, "-ffreestanding", "-O2"])
            .args([
                "-fno-omit-frame-pointer",
                "-Werror=incompatible-function-pointer-types",
            ])
            .arg("-I")
            .arg(sources)
            .arg("-c")
            .arg(sources.join(source))
            .arg("-o")
            .arg(&object))?;
        objects.push(object);
    }

    run(linker
        .args([
            "-arch",
            "x86_64",
            "-platform_version",
            "macos",
            "11.0",
            "11.0",
        ])
        .args(["-dylib", "-undefined", "dynamic_lookup"])
        .args(["-install_name", &format!("@rpath/{VR_LIBRARY}")])
        .args(&objects)
        .arg(sources.join("libSystem.tbd"))
        .arg("-o")
        .arg(library))
}

fn linker_command(compiler: &str) -> Option<Command> {
    if let Some(mach_o_linker) = env::var_os(LINKER_ENV)
        .map(PathBuf::from)
        .or_else(mach_o_linker_in_path)
        .or_else(|| mach_o_linker_beside(compiler))
    {
        return Some(Command::new(mach_o_linker));
    }

    let mut rust_linker = Command::new(rust_linker()?);
    rust_linker.args(["-flavor", "darwin"]);
    Some(rust_linker)
}

fn mach_o_linker_in_path() -> Option<PathBuf> {
    env::split_paths(&env::var_os("PATH")?)
        .map(|directory| directory.join(MACH_O_LINKER))
        .find(|candidate| candidate.is_file())
}

fn mach_o_linker_beside(compiler: &str) -> Option<PathBuf> {
    let output = Command::new(compiler)
        .arg(format!("-print-prog-name={MACH_O_LINKER}"))
        .output()
        .ok()?;
    let beside_compiler = PathBuf::from(String::from_utf8(output.stdout).ok()?.trim());
    beside_compiler.is_file().then_some(beside_compiler)
}

fn rust_linker() -> Option<PathBuf> {
    let rustc = env::var_os("RUSTC")?;
    let host = env::var("HOST").ok()?;
    let output = Command::new(rustc)
        .args(["--print", "sysroot"])
        .output()
        .ok()?;
    let sysroot = PathBuf::from(String::from_utf8(output.stdout).ok()?.trim());
    let rust_linker = sysroot
        .join("lib")
        .join("rustlib")
        .join(host)
        .join("bin")
        .join(RUST_LINKER);
    rust_linker.is_file().then_some(rust_linker)
}

fn run(command: &mut Command) -> Result<(), String> {
    let program = command.get_program().to_string_lossy().into_owned();
    let output = command
        .output()
        .map_err(|error| format!("couldn't run {program}: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    let errors = String::from_utf8_lossy(&output.stderr);
    let last_error = errors.lines().last().unwrap_or("no output");
    Err(format!("{program} failed: {last_error}"))
}
