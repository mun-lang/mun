extern crate cc;
#[macro_use]
extern crate lazy_static;
extern crate regex;
extern crate semver;

use regex::Regex;
use semver::Version;
use std::env;
use std::ffi::OsStr;
use std::io::{self, ErrorKind};
use std::path::PathBuf;
use std::process::Command;

lazy_static! {
    /// LLVM version used by this version of the crate.
    static ref CRATE_VERSION: Version = {
        let crate_version = Version::parse(env!("CARGO_PKG_VERSION"))
            .expect("Crate version is somehow not valid semver");
        Version {
            major: crate_version.major / 10,
            minor: crate_version.major % 10,
            .. crate_version
        }
    };

    static ref LLVM_CONFIG_BINARY_NAMES: Vec<String> = {
        vec![
            "llvm-config".into(),
            format!("llvm-config-{}", CRATE_VERSION.major),
            format!("llvm-config-{}.{}", CRATE_VERSION.major, CRATE_VERSION.minor),
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
        let binary_prefix_var = format!("LLVM_SYS_{}_PREFIX",
                                        env!("CARGO_PKG_VERSION_MAJOR"));
        if let Some(path) = env::var_os(&binary_prefix_var) {
            for binary_name in LLVM_CONFIG_BINARY_NAMES.iter() {
                let mut pb: PathBuf = path.clone().into();
                pb.push("bin");
                pb.push(binary_name);

                let ver = llvm_version(&pb)
                    .unwrap_or_else(|_| panic!("Failed to execute {:?}", &pb));
                if is_compatible_llvm(&ver) {
                    return pb;
                } else {
                    println!("LLVM binaries specified by {} are the wrong version.
                              (Found {}, need {}.)", binary_prefix_var, ver, *CRATE_VERSION);
                }
            }
        }

        println!("No suitable version of LLVM was found system-wide or pointed
                  to by {}.

                  Consider using `llvmenv` to compile an appropriate copy of LLVM, and
                  refer to the llvm-sys documentation for more information.

                  llvm-sys: https://crates.io/crates/llvm-sys
                  llvmenv: https://crates.io/crates/llvmenv", binary_prefix_var);
        panic!("Could not find a compatible version of LLVM");
    };
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
                    "Found LLVM version {} on PATH, but need {}.",
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

/// Check whether the given version of LLVM is blacklisted,
/// returning `Some(reason)` if it is.
fn is_blacklisted_llvm(llvm_version: &Version) -> Option<&'static str> {
    static BLACKLIST: &[(u64, u64, u64, &str)] = &[];

    let blacklist_var = format!(
        "LLVM_SYS_{}_IGNORE_BLACKLIST",
        env!("CARGO_PKG_VERSION_MAJOR")
    );
    if let Some(x) = env::var_os(&blacklist_var) {
        if &x == "YES" {
            println!(
                "cargo:warning=Ignoring blacklist entry for LLVM {}",
                llvm_version
            );
            return None;
        } else {
            println!(
                "cargo:warning={} is set but not exactly \"YES\"; blacklist is still honored.",
                &blacklist_var
            );
        }
    }

    for &(major, minor, patch, reason) in BLACKLIST.iter() {
        let bad_version = Version {
            major,
            minor,
            patch,
            pre: vec![],
            build: vec![],
        };

        if &bad_version == llvm_version {
            return Some(reason);
        }
    }
    None
}

/// Check whether the given LLVM version is compatible with this version of
/// the crate.
fn is_compatible_llvm(llvm_version: &Version) -> bool {
    if let Some(reason) = is_blacklisted_llvm(llvm_version) {
        println!(
            "Found LLVM {}, which is blacklisted: {}",
            llvm_version, reason
        );
        return false;
    }

    let strict = env::var_os(format!(
        "LLVM_SYS_{}_STRICT_VERSIONING",
        env!("CARGO_PKG_VERSION_MAJOR")
    ))
    .is_some()
        || cfg!(feature = "strict-versioning");
    if strict {
        llvm_version.major == CRATE_VERSION.major && llvm_version.minor == CRATE_VERSION.minor
    } else {
        llvm_version.major >= CRATE_VERSION.major
            || (llvm_version.major == CRATE_VERSION.major
                && llvm_version.minor >= CRATE_VERSION.minor)
    }
}

/// Get the output from running `llvm-config` with the given argument.
///
/// Lazily searches for or compiles LLVM as configured by the environment
/// variables.
fn llvm_config(arg: &str) -> String {
    llvm_config_ex(&*LLVM_CONFIG_PATH, arg).expect("Surprising failure from llvm-config")
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

/// Get the names of the dylibs required by LLVM, including the C++ standard
/// library.
fn get_system_libraries() -> Vec<String> {
    llvm_config("--system-libs")
        .split(&[' ', '\n'] as &[char])
        .filter(|s| !s.is_empty())
        .map(|flag| {
            if cfg!(target_env = "msvc") {
                // Same as --libnames, foo.lib
                assert!(flag.ends_with(".lib"));
                &flag[..flag.len() - 4]
            } else {
                // Linker flags style, -lfoo
                assert!(flag.starts_with("-l"));
                &flag[2..]
            }
        })
        .chain(get_system_libcpp())
        .map(str::to_owned)
        .collect::<Vec<String>>()
}

/// Get the library that must be linked for C++, if any.
fn get_system_libcpp() -> Option<&'static str> {
    if cfg!(target_env = "msvc") {
        // MSVC doesn't need an explicit one.
        None
    } else if cfg!(target_os = "macos") || cfg!(target_os = "freebsd") {
        // On OS X 10.9 and later, LLVM's libc++ is the default. On earlier
        // releases GCC's libstdc++ is default. Unfortunately we can't
        // reasonably detect which one we need (on older ones libc++ is
        // available and can be selected with -stdlib=lib++), so assume the
        // latest, at the cost of breaking the build on older OS releases
        // when LLVM was built against libstdc++.
        Some("c++")
    } else {
        // Otherwise assume GCC's libstdc++.
        // This assumption is probably wrong on some platforms, but would need
        // testing on them.
        Some("stdc++")
    }
}

/// Get the names of libraries to link against.
fn get_link_libraries() -> Vec<String> {
    // Using --libnames in conjunction with --libdir is particularly important
    // for MSVC when LLVM is in a path with spaces, but it is generally less of
    // a hack than parsing linker flags output from --libs and --ldflags.
    llvm_config("--libnames")
        .split(&[' ', '\n'] as &[char])
        .filter(|s| !s.is_empty())
        .map(|name| {
            // --libnames gives library filenames. Extract only the name that
            // we need to pass to the linker.
            if cfg!(target_env = "msvc") {
                // LLVMfoo.lib
                assert!(name.ends_with(".lib"));
                &name[..name.len() - 4]
            } else {
                // libLLVMfoo.a
                assert!(name.starts_with("lib") && name.ends_with(".a"));
                &name[3..name.len() - 2]
            }
        })
        .map(str::to_owned)
        .collect::<Vec<String>>()
}

fn get_llvm_cxxflags() -> String {
    let output = llvm_config("--cxxflags");

    // llvm-config includes cflags from its own compilation with --cflags that
    // may not be relevant to us. In particularly annoying cases, these might
    // include flags that aren't understood by the default compiler we're
    // using. Unless requested otherwise, clean CFLAGS of options that are
    // known to be possibly-harmful.
    let no_clean = env::var_os(format!(
        "LLVM_SYS_{}_NO_CLEAN_CFLAGS",
        env!("CARGO_PKG_VERSION_MAJOR")
    ))
    .is_some();
    if no_clean || cfg!(target_env = "msvc") {
        // MSVC doesn't accept -W... options, so don't try to strip them and
        // possibly strip something that should be retained. Also do nothing if
        // the user requests it.
        return output;
    }

    llvm_config("--cxxflags")
        .split(&[' ', '\n'][..])
        .filter(|word| !word.starts_with("-W"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_llvm_debug() -> bool {
    // Has to be either Debug or Release
    llvm_config("--build-mode").contains("Debug")
}

fn main() {
    // Build the extra wrapper functions.
    std::env::set_var("CXXFLAGS", get_llvm_cxxflags());
    cc::Build::new()
        .cpp(true)
        .file("wrapper/lld-c.cpp")
        .compile("lldwrapper");

    if cfg!(feature = "no-llvm-linking") {
        return;
    }

    let libdir = llvm_config("--libdir");

    // Export information to other crates
    println!("cargo:config_path={}", LLVM_CONFIG_PATH.display()); // will be DEP_LLVM_CONFIG_PATH
    println!("cargo:libdir={}", libdir); // DEP_LLVM_LIBDIR

    // Link LLVM libraries
    println!("cargo:rustc-link-search=native={}", libdir);
    let blacklist = vec!["LLVMLineEditor"];
    for name in get_link_libraries()
        .iter()
        .filter(|n| !blacklist.iter().any(|blacklisted| n.contains(*blacklisted)))
    {
        println!("cargo:rustc-link-lib=static={}", name);
    }

    // Link system libraries
    for name in get_system_libraries() {
        println!("cargo:rustc-link-lib=dylib={}", name);
    }

    let use_debug_msvcrt = env::var_os(format!(
        "LLVM_SYS_{}_USE_DEBUG_MSVCRT",
        env!("CARGO_PKG_VERSION_MAJOR")
    ))
    .is_some();
    if cfg!(target_env = "msvc") && (use_debug_msvcrt || is_llvm_debug()) {
        println!("cargo:rustc-link-lib=msvcrtd");
    }

    // Link libffi if the user requested this workaround.
    // See https://bitbucket.org/tari/llvm-sys.rs/issues/12/
    let force_ffi = env::var_os(format!(
        "LLVM_SYS_{}_FFI_WORKAROUND",
        env!("CARGO_PKG_VERSION_MAJOR")
    ))
    .is_some();
    if force_ffi {
        println!("cargo:rustc-link-lib=dylib=ffi");
    }

    println!("cargo:rustc-link-lib=static=lldCOFF");
    println!("cargo:rustc-link-lib=static=lldCommon");
    println!("cargo:rustc-link-lib=static=lldCore");
    println!("cargo:rustc-link-lib=static=lldDriver");
    println!("cargo:rustc-link-lib=static=lldELF");
    println!("cargo:rustc-link-lib=static=lldMachO");
    println!("cargo:rustc-link-lib=static=lldMinGW");
    println!("cargo:rustc-link-lib=static=lldReaderWriter");
    println!("cargo:rustc-link-lib=static=lldWasm");
    println!("cargo:rustc-link-lib=static=lldYAML");

    if cfg!(not(target_os = "windows")) {
        println!("cargo:rustc-link-lib=dylib=ffi");
    }
}
