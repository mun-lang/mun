use mun_target::spec;
use mun_target::spec::LinkerFlavor;
use std::path::Path;

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
    fn finalize(&mut self) -> Result<(), String>;
}

struct LdLinker {
    args: Vec<String>,
}

impl LdLinker {
    fn new(_target: &spec::Target) -> Self {
        LdLinker {
            args: Vec::default(),
        }
    }
}

impl Linker for LdLinker {
    fn add_object(&mut self, path: &Path) {
        self.args.push(path.to_string_lossy().to_string());
    }

    fn build_shared_object(&mut self, path: &Path) {
        // Link as dynamic library
        self.args.push("--shared".to_string());

        // Specify output path
        self.args.push("-o".to_string());
        self.args.push(path.to_str().unwrap().to_string());
    }

    fn finalize(&mut self) -> Result<(), String> {
        mun_lld::link(mun_lld::LldFlavor::Elf, &self.args).ok()
    }
}

struct Ld64Linker {
    args: Vec<String>,
}

impl Ld64Linker {
    fn new(target: &spec::Target) -> Self {
        Ld64Linker {
            args: vec![format!("-arch {}", &target.arch)],
        }
    }
}

impl Linker for Ld64Linker {
    fn add_object(&mut self, path: &Path) {
        self.args.push(path.to_string_lossy().to_string());
    }

    fn build_shared_object(&mut self, path: &Path) {
        // Link as dynamic library
        self.args.push("-dylib".to_string());
        self.args.push("-lsystem".to_string());

        // Specify output path
        self.args.push("-o".to_string());
        self.args.push(path.to_str().unwrap().to_string());
    }

    fn finalize(&mut self) -> Result<(), String> {
        mun_lld::link(mun_lld::LldFlavor::MachO, &self.args).ok()
    }
}

struct MsvcLinker {
    args: Vec<String>,
}

impl MsvcLinker {
    fn new(_target: &spec::Target) -> Self {
        MsvcLinker {
            args: Vec::default(),
        }
    }
}

impl Linker for MsvcLinker {
    fn add_object(&mut self, path: &Path) {
        self.args.push(path.to_string_lossy().to_string());
    }

    fn build_shared_object(&mut self, path: &Path) {
        self.args.push("/DLL".to_string());
        self.args.push("/NOENTRY".to_string());
        self.args.push("/EXPORT:get_info".to_string());
        self.args.push(format!(
            "/IMPLIB:{}",
            path.with_extension("dll.lib").to_string_lossy()
        ));
        self.args.push(format!("/OUT:{}", path.to_string_lossy()));
    }

    fn finalize(&mut self) -> Result<(), String> {
        mun_lld::link(mun_lld::LldFlavor::Coff, &self.args).ok()
    }
}
