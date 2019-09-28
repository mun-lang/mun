use rustc_hash::FxHashMap;

/// `FileId` is an integer which uniquely identifies a file. File paths are messy and
/// system-dependent, so most of the code should work directly with `FileId`, without inspecting the
/// path. The mapping between `FileId` and path and `SourceRoot` is constant. A file rename is
/// represented as a pair of deletion/creation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileId(pub u32);

/// `PackageInput` contains which files belong to a specific package.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PackageInput {
    arena: FxHashMap<ModuleId, ModuleData>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleData {
    file_id: FileId,
}

impl ModuleData {
    fn new(file_id: FileId) -> ModuleData {
        ModuleData { file_id }
    }
}

impl PackageInput {
    pub fn add_module(&mut self, file_id: FileId) -> ModuleId {
        let module_id = ModuleId(self.arena.len() as u32);
        self.arena.insert(module_id, ModuleData::new(file_id));
        module_id
    }

    pub fn modules<'a>(&'a self) -> impl Iterator<Item = FileId> + 'a {
        self.arena.values().map(|module| module.file_id)
    }
}
