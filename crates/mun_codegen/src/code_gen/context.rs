use crate::ty::HirTypeCache;

pub struct CodeGenContext<'db> {
    /// The Salsa HIR database
    pub db: &'db dyn mun_hir::HirDatabase,

    /// A mapping from Rust types' full path (without lifetime) to inkwell types
    // pub rust_types: RefCell<HashMap<&'static str, StructType<'ink>>>,

    /// A mapping from HIR types to LLVM struct types
    pub hir_types: HirTypeCache<'db>,
}

impl<'db> CodeGenContext<'db> {
    /// Constructs a new `CodeGenContext` from an LLVM context and a
    /// `CodeGenDatabase`.
    pub fn new(db: &'db dyn mun_hir::HirDatabase) -> Self {
        Self {
            // rust_types: RefCell::new(HashMap::default()),
            hir_types: HirTypeCache::new(db),
            db,
        }
    }
}
