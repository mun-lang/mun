use crate::apple::get_apple_sdk_root;
use mun_abi as abi;
use mun_target::spec;
use mun_target::spec::LinkerFlavor;
use std::borrow::Cow;
use std::fmt;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LinkerError {
    /// Error emitted by the linker
    LinkError(String),

    /// Error in path conversion
    PathError(PathBuf),

    /// Could not locate platform SDK
    PlatformSdkMissing(String),
}

impl fmt::Display for LinkerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            LinkerError::LinkError(e) => write!(f, "{e}"),
            LinkerError::PathError(path) => write!(
                f,
                "path contains invalid UTF-8 characters: {}",
                path.display()
            ),
            LinkerError::PlatformSdkMissing(err) => {
                write!(f, "could not find platform sdk: {err}")
            }
        }
    }
}

pub fn create_with_target(target: &spec::Target) -> Box<dyn Linker> {
    match target.options.linker_flavor {
        LinkerFlavor::Ld => Box::new(LdLinker::new(target)),
        LinkerFlavor::Ld64 => Box::new(Ld64Linker::new(target)),
        LinkerFlavor::Msvc => Box::new(MsvcLinker::new(target)),
    }
}

pub trait Linker {
    fn add_object(&mut self, path: &Path) -> Result<(), LinkerError>;
    fn build_shared_object(&mut self, path: &Path) -> Result<(), LinkerError>;
    fn finalize(&mut self) -> Result<(), LinkerError>;
}

struct LdLinker {
    args: Vec<String>,
}

impl LdLinker {
    fn new(target: &spec::Target) -> Self {
        LdLinker {
            args: target
                .options
                .pre_link_args
                .iter()
                .cloned()
                .map(Cow::into_owned)
                .collect(),
        }
    }
}

impl Linker for LdLinker {
    fn add_object(&mut self, path: &Path) -> Result<(), LinkerError> {
        let path_str = path
            .to_str()
            .ok_or_else(|| LinkerError::PathError(path.to_owned()))?
            .to_owned();
        self.args.push(path_str);
        Ok(())
    }

    fn build_shared_object(&mut self, path: &Path) -> Result<(), LinkerError> {
        let path_str = path
            .to_str()
            .ok_or_else(|| LinkerError::PathError(path.to_owned()))?;

        // Link as dynamic library
        self.args.push("--shared".to_owned());

        // Specify output path
        self.args.push("-o".to_owned());
        self.args.push(path_str.to_owned());

        Ok(())
    }

    fn finalize(&mut self) -> Result<(), LinkerError> {
        lld_rs::link(lld_rs::LldFlavor::Elf, &self.args)
            .ok()
            .map_err(LinkerError::LinkError)
    }
}

struct Ld64Linker {
    args: Vec<String>,
    target: spec::Target,
}

impl Ld64Linker {
    fn new(target: &spec::Target) -> Self {
        let args = target
            .options
            .pre_link_args
            .iter()
            .cloned()
            .map(Cow::into_owned)
            .collect();

        Ld64Linker {
            args,
            target: target.clone(),
        }
    }

    fn add_apple_sdk(&mut self) -> Result<(), LinkerError> {
        let arch = &self.target.arch;
        let os = &self.target.options.os;
        let llvm_target = &self.target.llvm_target;

        let sdk_name = match (arch.as_ref(), os.as_ref()) {
            ("aarch64", "tvos") => "appletvos",
            ("x86_64", "tvos") => "appletvsimulator",
            ("aarch64", "ios") if llvm_target.contains("macabi") => "macosx",
            ("aarch64", "ios") if llvm_target.ends_with("-simulator") => "iphonesimulator",
            ("x86_64", "ios") if llvm_target.contains("macabi") => "macosx",
            ("arm" | "aarch64", "ios") => "iphoneos",
            ("x86" | "x86_64", "ios") => "iphonesimulator",
            ("aarch64", "watchos") if llvm_target.ends_with("-simulator") => "watchsimulator",
            ("x86_64", "watchos") => "watchsimulator",
            ("aarch64" | "arm" | "arm64_32", "watchos") => "watchos",
            (_, "macos") => "macosx",
            _ => {
                return Err(LinkerError::PlatformSdkMissing(format!(
                    "unsupported arch `{arch}` for os `{os}`"
                )));
            }
        };

        let sdk_root = get_apple_sdk_root(sdk_name).map_err(LinkerError::PlatformSdkMissing)?;
        self.args.push(String::from("-syslibroot"));
        self.args.push(format!("{}", sdk_root.display()));
        Ok(())
    }
}

impl Linker for Ld64Linker {
    fn add_object(&mut self, path: &Path) -> Result<(), LinkerError> {
        let path_str = path
            .to_str()
            .ok_or_else(|| LinkerError::PathError(path.to_owned()))?
            .to_owned();
        self.args.push(path_str);
        Ok(())
    }

    fn build_shared_object(&mut self, path: &Path) -> Result<(), LinkerError> {
        let path_str = path
            .to_str()
            .ok_or_else(|| LinkerError::PathError(path.to_owned()))?;

        let filename_str = path
            .file_name()
            .expect("path must have a filename")
            .to_str()
            .ok_or_else(|| LinkerError::PathError(path.to_owned()))?;

        // Link as dynamic library
        self.args.push("-dylib".to_owned());

        self.add_apple_sdk()?;
        self.args.push("-lSystem".to_owned());

        // Specify output path
        self.args.push("-o".to_owned());
        self.args.push(path_str.to_owned());

        // Ensure that the `install_name` is not a full path as it is used as a unique identifier on
        // MacOS
        self.args.push("-install_name".to_owned());
        self.args.push(filename_str.to_owned());

        Ok(())
    }

    fn finalize(&mut self) -> Result<(), LinkerError> {
        lld_rs::link(lld_rs::LldFlavor::MachO, &self.args)
            .ok()
            .map_err(LinkerError::LinkError)
    }
}

struct MsvcLinker {
    args: Vec<String>,
}

impl MsvcLinker {
    fn new(target: &spec::Target) -> Self {
        MsvcLinker {
            args: target
                .options
                .pre_link_args
                .iter()
                .cloned()
                .map(Cow::into_owned)
                .collect(),
        }
    }
}

impl Linker for MsvcLinker {
    fn add_object(&mut self, path: &Path) -> Result<(), LinkerError> {
        let path_str = path
            .to_str()
            .ok_or_else(|| LinkerError::PathError(path.to_owned()))?
            .to_owned();
        self.args.push(path_str);
        Ok(())
    }

    fn build_shared_object(&mut self, path: &Path) -> Result<(), LinkerError> {
        let dll_path_str = path
            .to_str()
            .ok_or_else(|| LinkerError::PathError(path.to_owned()))?;

        let dll_lib_path_str = path
            .to_str()
            .ok_or_else(|| LinkerError::PathError(path.to_owned()))?;

        self.args.push("/DLL".to_owned());
        self.args.push("/NOENTRY".to_owned());
        self.args.push(format!("/EXPORT:{}", abi::GET_INFO_FN_NAME));
        self.args
            .push(format!("/EXPORT:{}", abi::GET_VERSION_FN_NAME));
        self.args
            .push(format!("/EXPORT:{}", abi::SET_ALLOCATOR_HANDLE_FN_NAME));
        self.args.push(format!("/IMPLIB:{dll_lib_path_str}"));
        self.args.push(format!("/OUT:{dll_path_str}"));
        Ok(())
    }

    fn finalize(&mut self) -> Result<(), LinkerError> {
        lld_rs::link(lld_rs::LldFlavor::Coff, &self.args)
            .ok()
            .map_err(LinkerError::LinkError)
    }
}
