use regex::Regex;
use semver::Version;
use std::{env, ffi::OsStr, fs, io, io::ErrorKind, path::Path, path::PathBuf, process::Command};

#[macro_use]
extern crate lazy_static;

const LLVM_VERSION_MAJOR: u32 = 7;
const LLVM_VERSION_MINOR: u32 = 0;

// Environment variables that can guide compilation
//
// When adding new ones, they should also be added to main() to force a
// rebuild if they are changed.

lazy_static! {
    /// A single path to search for LLVM in (containing bin/llvm-config)
    static ref ENV_LLVM_PREFIX: String =
        format!("LLVM_SYS_{}_PREFIX", LLVM_VERSION_MAJOR*10);

    /// If exactly "YES", ignore the version blacklist
    static ref ENV_IGNORE_BLACKLIST: String =
        format!("LLVM_SYS_{}_IGNORE_BLACKLIST", LLVM_VERSION_MAJOR*10);

    /// If set, enforce precise correspondence between crate and binary versions.
    static ref ENV_STRICT_VERSIONING: String =
        format!("LLVM_SYS_{}_STRICT_VERSIONING", LLVM_VERSION_MAJOR*10);

    /// If set, do not attempt to strip irrelevant options for llvm-config --cflags
    static ref ENV_NO_CLEAN_CFLAGS: String =
        format!("LLVM_SYS_{}_NO_CLEAN_CFLAGS", LLVM_VERSION_MAJOR*10);

    /// If set and targeting MSVC, force the debug runtime library
    static ref ENV_USE_DEBUG_MSVCRT: String =
        format!("LLVM_SYS_{}_USE_DEBUG_MSVCRT", LLVM_VERSION_MAJOR*10);

    /// If set, always link against libffi
    static ref ENV_FORCE_FFI: String =
        format!("LLVM_SYS_{}_FFI_WORKAROUND", LLVM_VERSION_MAJOR*10);
}

lazy_static! {
    /// LLVM version used by this version of the crate.
    static ref CRATE_VERSION: Version = {
        Version::parse(&format!("{}.{}.0",LLVM_VERSION_MAJOR, LLVM_VERSION_MINOR))
            .expect("Crate version is somehow not valid semver")
    };

    static ref LLVM_CONFIG_BINARY_NAMES: Vec<String> = {
        vec![
            format!("llvm-config{}", std::env::consts::EXE_SUFFIX),
            format!("llvm-config-{}{}", LLVM_VERSION_MAJOR, std::env::consts::EXE_SUFFIX),
            format!("llvm-config-{}.{}{}", LLVM_VERSION_MAJOR, LLVM_VERSION_MINOR, std::env::consts::EXE_SUFFIX),
        ]
    };

    /// Filesystem path to an llvm-config binary for the correct version.
    static ref LLVM_CONFIG_PATH: PathBuf = {
        // Try llvm-config via PATH first.
        if let Some(name) = locate_system_llvm_config() {
            return name.into();
        } else {
            println!("Didn't find usable system-wide LLVM.");
        }

        // Did the user give us a binary path to use? If yes, try
        // to use that and fail if it doesn't work.
        if let Some(path) = env::var_os(&*ENV_LLVM_PREFIX) {
            for binary_name in LLVM_CONFIG_BINARY_NAMES.iter() {
                let mut pb: PathBuf = path.clone().into();
                pb.push("bin");
                pb.push(binary_name);

                let ver = llvm_version(&pb)
                    .expect(&format!("Failed to execute {:?}", &pb));
                if is_compatible_llvm(&ver) {
                    return pb;
                } else {
                    println!("LLVM binaries specified by {} are the wrong version.
                              (Found {}, need {}.)", *ENV_LLVM_PREFIX, ver, *CRATE_VERSION);
                }
            }
        }

        println!("No suitable version of LLVM was found system-wide or pointed
                  to by {}.
                  
                  Consider using `llvmenv` to compile an appropriate copy of LLVM, and
                  refer to the llvm-sys documentation for more information.
                  
                  llvm-sys: https://crates.io/crates/llvm-sys
                  llvmenv: https://crates.io/crates/llvmenv", *ENV_LLVM_PREFIX);
        panic!("Could not find a compatible version of LLVM");
    };
}

/// Get the output from running `llvm-config` with the given argument.
///
/// Lazily searches for or compiles LLVM as configured by the environment
/// variables.
fn llvm_config(arg: &str) -> String {
    llvm_config_ex(&*LLVM_CONFIG_PATH, arg).expect("Surprising failure from llvm-config")
}

/// Try to find a system-wide version of llvm-config that is compatible with
/// this crate.
///
/// Returns None on failure.
fn locate_system_llvm_config() -> Option<&'static str> {
    for binary_name in LLVM_CONFIG_BINARY_NAMES.iter() {
        match llvm_version(binary_name) {
            Ok(ref version) if is_compatible_llvm(version) => {
                // Compatible version found. Nice.
                return Some(binary_name);
            }
            Ok(version) => {
                // Version mismatch. Will try further searches, but warn that
                // we're not using the system one.
                println!(
                    "Found LLVM version {} on PATH, but need {}",
                    version, *CRATE_VERSION
                );
            }
            Err(ref e) if e.kind() == ErrorKind::NotFound => {
                // Looks like we failed to execute any llvm-config. Keep
                // searching.
            }
            // Some other error, probably a weird failure. Give up.
            Err(e) => panic!("Failed to search PATH for llvm-config: {}", e),
        }
    }

    None
}

/// Check whether the given LLVM version is compatible with this version of
/// the crate.
fn is_compatible_llvm(llvm_version: &Version) -> bool {
    //    if let Some(reason) = is_blacklisted_llvm(llvm_version) {
    //        println!(
    //            "Found LLVM {}, which is blacklisted: {}",
    //            llvm_version, reason
    //        );
    //        return false;
    //    }

    let strict =
        env::var_os(&*ENV_STRICT_VERSIONING).is_some() || cfg!(feature = "strict-versioning");
    if strict {
        llvm_version.major == CRATE_VERSION.major && llvm_version.minor == CRATE_VERSION.minor
    } else {
        llvm_version.major >= CRATE_VERSION.major
            || (llvm_version.major == CRATE_VERSION.major
                && llvm_version.minor >= CRATE_VERSION.minor)
    }
}

/// Invoke the specified binary as llvm-config.
///
/// Explicit version of the `llvm_config` function that bubbles errors
/// up.
fn llvm_config_ex<S: AsRef<OsStr>>(binary: S, arg: &str) -> io::Result<String> {
    Command::new(binary)
        .arg(arg)
        .arg("--link-static") // Don't use dylib for >= 3.9
        .output()
        .map(|output| {
            String::from_utf8(output.stdout).expect("Output from llvm-config was not valid UTF-8")
        })
}

/// Get the LLVM version using llvm-config.
fn llvm_version<S: AsRef<OsStr>>(binary: S) -> io::Result<Version> {
    let version_str = llvm_config_ex(binary.as_ref(), "--version")?;

    // LLVM isn't really semver and uses version suffixes to build
    // version strings like '3.8.0svn', so limit what we try to parse
    // to only the numeric bits.
    let re = Regex::new(r"^(?P<major>\d+)\.(?P<minor>\d+)(?:\.(?P<patch>\d+))??").unwrap();
    let c = re
        .captures(&version_str)
        .expect("Could not determine LLVM version from llvm-config.");

    // some systems don't have a patch number but Version wants it so we just append .0 if it isn't
    // there
    let s = match c.name("patch") {
        None => format!("{}.0", &c[0]),
        Some(_) => c[0].to_string(),
    };
    Ok(Version::parse(&s).unwrap())
}

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let bindir = llvm_config("--bindir").trim().to_string();

    let lld_path = Path::new(&bindir)
        .join(format!("lld{}", std::env::consts::EXE_SUFFIX))
        .canonicalize()
        .expect(&format!(
            "unable to locate 'lld{}' in {}",
            std::env::consts::EXE_SUFFIX,
            bindir
        ));
    let lld_output_path = Path::new(&out_dir)
        .join("../../..")
        .canonicalize()
        .unwrap()
        .join(format!("lld{}", std::env::consts::EXE_SUFFIX));
    fs::copy(lld_path, lld_output_path).unwrap();
}
