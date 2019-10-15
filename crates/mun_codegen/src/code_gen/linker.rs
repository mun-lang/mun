use mun_target::spec;
use mun_target::spec::LinkerFlavor;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;
use std::process;
use std::process::Command;

pub fn create_with_target(target: &spec::Target) -> Box<dyn Linker> {
    match target.linker_flavor {
        LinkerFlavor::Ld => Box::new(LdLinker::new(target)),
        LinkerFlavor::Ld64 => Box::new(Ld64Linker::new(target)),
        LinkerFlavor::Msvc => Box::new(MsvcLinker::new(target)),
    }
}

pub trait Linker {
    fn add_object(&mut self, path: &Path);
    fn build_shared_object(&mut self, path: &Path);
    fn finalize(&mut self) -> process::Command;
}

struct LdLinker {
    cmd: process::Command,
}

struct Ld64Linker {
    cmd: process::Command,
}

struct MsvcLinker {
    cmd: process::Command,
}

impl LdLinker {
    fn new(_target: &spec::Target) -> Self {
        let mut cmd = process::Command::new(LLD_PATH.as_os_str());
        cmd.arg("-flavor");
        cmd.arg("ld");

        LdLinker { cmd }
    }
}

impl Ld64Linker {
    fn new(target: &spec::Target) -> Self {
        let mut cmd = process::Command::new(LLD_PATH.as_os_str());
        cmd.arg("-flavor");
        cmd.arg("ld64");

        cmd.arg("-arch");
        cmd.arg(&target.arch);

        Ld64Linker { cmd }
    }
}

lazy_static! {
    static ref LLD_PATH: PathBuf = {
        let path = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();

        let mut binary_dir: &Path = &path;
        let mut binary_path;
        loop {
            binary_path = binary_dir.to_path_buf();
            binary_path.push(format!("lld{}", std::env::consts::EXE_SUFFIX));
            if binary_path.exists() {
                break;
            }
            binary_dir = binary_dir.parent().expect("could not find lld");
        }
        binary_path
    };
}

impl MsvcLinker {
    fn new(_target: &spec::Target) -> Self {
        let mut cmd = process::Command::new(LLD_PATH.as_os_str());
        cmd.arg("-flavor");
        cmd.arg("link");

        MsvcLinker { cmd }
    }
}

impl Linker for LdLinker {
    fn add_object(&mut self, path: &Path) {
        self.cmd.arg(path);
    }

    fn build_shared_object(&mut self, path: &Path) {
        // Link as dynamic library
        self.cmd.arg("--shared");
        //        self.cmd.arg("--apply-dynamic-relocs");
        //        self.cmd.arg("--pie");

        // Specify output path
        self.cmd.arg("-o");
        self.cmd.arg(path);
    }

    fn finalize(&mut self) -> process::Command {
        ::std::mem::replace(&mut self.cmd, Command::new(""))
    }
}

impl Linker for Ld64Linker {
    fn add_object(&mut self, path: &Path) {
        self.cmd.arg(path);
    }

    fn build_shared_object(&mut self, path: &Path) {
        // Link as dynamic library
        self.cmd.arg("-dylib");
        self.cmd.arg("-lsystem");

        // Specify output path
        self.cmd.arg("-o");
        self.cmd.arg(path);
    }

    fn finalize(&mut self) -> process::Command {
        ::std::mem::replace(&mut self.cmd, Command::new(""))
    }
}

impl Linker for MsvcLinker {
    fn add_object(&mut self, path: &Path) {
        self.cmd.arg(path);
    }

    fn build_shared_object(&mut self, path: &Path) {
        self.cmd.arg("/DLL");
        self.cmd.arg("/NOENTRY");
        self.cmd.arg("/EXPORT:get_info");

        let mut arg = OsString::from("/IMPLIB:");
        arg.push(path.with_extension("dll.lib"));
        self.cmd.arg(arg);

        let mut arg = OsString::from("/OUT:");
        arg.push(path);
        self.cmd.arg(arg);
    }

    fn finalize(&mut self) -> process::Command {
        ::std::mem::replace(&mut self.cmd, Command::new(""))
    }
}
