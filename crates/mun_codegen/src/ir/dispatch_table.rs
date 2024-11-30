use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use inkwell::{
    context::Context,
    module::Module,
    targets::TargetData,
    types::{BasicTypeEnum, FunctionType},
    values::{BasicValueEnum, CallableValue},
};
use mun_hir::{Body, Expr, ExprId, HirDatabase, InferenceResult};
use rustc_hash::FxHashSet;

use crate::{
    intrinsics::Intrinsic,
    ir::{function, ty::HirTypeCache},
    module_group::ModuleGroup,
    type_info::{HasStaticTypeId, TypeId},
};

/// A dispatch table in IR is a struct that contains pointers to all functions
/// that are called from code. In C terms it looks something like this:
/// ```c
/// struct DispatchTable {
///     int(*foo)(int, int);
///     // .. etc
/// } dispatchTable;
/// ```
///
/// The dispatch table is used to add a patchable indirection when calling a
/// function from IR. The `DispatchTable` is exposed to the Runtime which fills
/// the structure with valid pointers to functions. This basically enables all
/// hot reloading within Mun.
#[derive(Debug, Eq, PartialEq)]
pub struct DispatchTable<'ink> {
    // The LLVM context in which all LLVM types live
    context: &'ink Context,
    // The target for which to create the dispatch table
    target: TargetData,
    // This contains the function that map to the DispatchTable struct fields
    function_to_idx: HashMap<mun_hir::Function, usize>,
    // Prototype to function index
    prototype_to_idx: HashMap<FunctionPrototype, usize>,
    // This contains an ordered list of all the function in the dispatch table
    entries: Vec<DispatchableFunction>,
    // Contains a reference to the global value containing the DispatchTable
    table_ref: Option<inkwell::values::GlobalValue<'ink>>,
    //
    table_type: Option<inkwell::types::StructType<'ink>>,
}

/// A `FunctionPrototype` defines a unique signature that can be added to the
/// dispatch table.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct FunctionPrototype {
    pub name: String,
    pub arg_types: Vec<Arc<TypeId>>,
    pub ret_type: Arc<TypeId>,
}

/// A `DispatchableFunction` is an entry in the dispatch table that may or may
/// not be pointing to an existing `mun_hir` function.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DispatchableFunction {
    pub prototype: FunctionPrototype,
    pub mun_hir: Option<mun_hir::Function>,
}

impl<'ink> DispatchTable<'ink> {
    /// Returns whether the `DispatchTable` contains the specified `function`.
    pub fn contains(&self, function: mun_hir::Function) -> bool {
        self.function_to_idx.contains_key(&function)
    }

    /// Returns a slice containing all the functions in the dispatch table.
    pub fn entries(&self) -> &[DispatchableFunction] {
        &self.entries
    }

    /// Generate a function lookup through the `DispatchTable`, equivalent to
    /// something along the lines of: `dispatchTable[i]`, where i is the
    /// index of the function and `dispatchTable` is a struct
    pub fn gen_function_lookup(
        &self,
        db: &dyn HirDatabase,
        table_ref: Option<inkwell::values::GlobalValue<'ink>>,
        builder: &inkwell::builder::Builder<'ink>,
        function: mun_hir::Function,
    ) -> CallableValue<'ink> {
        let function_name = function.name(db).to_string();

        // Get the index of the function
        let index = *self
            .function_to_idx
            .get(&function)
            .expect("unknown function");

        Self::gen_function_lookup_by_index(table_ref, builder, &function_name, index)
    }

    /// Generates a function lookup through the `DispatchTable`, equivalent to
    /// something along the lines of: `dispatchTable[i]`, where i is the
    /// index of the intrinsic and `dispatchTable` is a struct
    pub fn gen_intrinsic_lookup(
        &self,
        table_ref: Option<inkwell::values::GlobalValue<'ink>>,
        builder: &inkwell::builder::Builder<'ink>,
        intrinsic: &impl Intrinsic,
    ) -> CallableValue<'ink> {
        let prototype = intrinsic.prototype();

        // Get the index of the intrinsic
        let index = *self
            .prototype_to_idx
            .get(&prototype)
            .expect("unknown function");

        Self::gen_function_lookup_by_index(table_ref, builder, &prototype.name, index)
    }

    /// Generates a function lookup through the `DispatchTable`, equivalent to
    /// something along the lines of: `dispatchTable[i]`, where i is the
    /// index and `dispatchTable` is a struct
    fn gen_function_lookup_by_index(
        table_ref: Option<inkwell::values::GlobalValue<'ink>>,
        builder: &inkwell::builder::Builder<'ink>,
        function_name: &str,
        index: usize,
    ) -> CallableValue<'ink> {
        // Get the internal table reference
        let table_ref = table_ref.expect("no dispatch table defined");

        // Create an expression that finds the associated field in the table and returns
        // this as a pointer access
        let ptr_to_function_ptr = builder
            .build_struct_gep(
                table_ref.as_pointer_value(),
                index as u32,
                &format!("{function_name}_ptr_ptr"),
            )
            .unwrap_or_else(|_| {
                panic!("could not get {function_name} (index: {index}) from dispatch table")
            });

        builder
            .build_load(ptr_to_function_ptr, &format!("{function_name}_ptr"))
            .into_pointer_value()
            .try_into()
            .expect("Pointer value is not a valid function pointer.")
    }

    /// Returns the value that represents the dispatch table in IR or `None` if
    /// no table was generated.
    pub fn global_value(&self) -> Option<&inkwell::values::GlobalValue<'ink>> {
        self.table_ref.as_ref()
    }

    /// Returns the IR type of the dispatch table's global value, if it exists.
    pub fn ty(&self) -> Option<inkwell::types::StructType<'ink>> {
        self.table_type
    }
}

/// A struct that can be used to build the dispatch table from HIR.
pub(crate) struct DispatchTableBuilder<'db, 'ink, 't> {
    db: &'db dyn HirDatabase,
    // The LLVM context in which all LLVM types live
    context: &'ink Context,
    // The module in which all values live
    module: &'t Module<'ink>,
    // The target for which to create the dispatch table
    target_data: TargetData,
    // Converts HIR ty's to inkwell types
    hir_types: &'t HirTypeCache<'db, 'ink>,
    // This contains the functions that map to the DispatchTable struct fields
    function_to_idx: HashMap<mun_hir::Function, usize>,
    // Prototype to function index
    prototype_to_idx: HashMap<FunctionPrototype, usize>,
    // These are *all* called functions in the modules
    entries: Vec<TypedDispatchableFunction<'ink>>,
    // Contains a reference to the global value containing the DispatchTable
    table_ref: Option<inkwell::values::GlobalValue<'ink>>,
    // This is the actual DispatchTable type
    table_type: inkwell::types::StructType<'ink>,
    // The group of modules for which the dispatch table is being build
    module_group: &'t ModuleGroup,
    // The set of modules that is referenced
    referenced_modules: FxHashSet<mun_hir::Module>,
}

struct TypedDispatchableFunction<'ink> {
    function: DispatchableFunction,
    ir_type: FunctionType<'ink>,
}

impl<'db, 'ink, 't> DispatchTableBuilder<'db, 'ink, 't> {
    /// Creates a new builder that can generate a dispatch function.
    pub fn new(
        context: &'ink Context,
        target_data: TargetData,
        db: &'db dyn HirDatabase,
        module: &'t Module<'ink>,
        intrinsics: &BTreeMap<FunctionPrototype, FunctionType<'ink>>,
        hir_types: &'t HirTypeCache<'db, 'ink>,
        module_group: &'t ModuleGroup,
    ) -> Self {
        let mut table = Self {
            db,
            context,
            module,
            target_data,
            function_to_idx: HashMap::default(),
            prototype_to_idx: HashMap::default(),
            entries: Vec::default(),
            table_ref: None,
            table_type: context.opaque_struct_type("DispatchTable"),
            hir_types,
            module_group,
            referenced_modules: FxHashSet::default(),
        };

        if !intrinsics.is_empty() {
            table.ensure_table_ref();

            // Use a `BTreeMap` to guarantee deterministically ordered output
            for (prototype, ir_type) in intrinsics.iter() {
                let index = table.entries.len();
                table.entries.push(TypedDispatchableFunction {
                    function: DispatchableFunction {
                        prototype: prototype.clone(),
                        mun_hir: None,
                    },
                    ir_type: *ir_type,
                });

                table.prototype_to_idx.insert(prototype.clone(), index);
            }
        }
        table
    }

    /// Creates the global dispatch table in the module if it does not exist.
    fn ensure_table_ref(&mut self) {
        if self.table_ref.is_none() {
            self.table_ref = Some(
                self.module
                    .add_global(self.table_type, None, "dispatchTable"),
            );
        }
    }

    /// Collects call expression from the given expression and sub expressions.
    fn collect_expr(&mut self, expr_id: ExprId, body: &Arc<Body>, infer: &InferenceResult) {
        let expr = &body[expr_id];

        // If this expression is a call, store it in the dispatch table
        if let Expr::Call { callee, .. } = expr {
            match infer[*callee].as_callable_def() {
                Some(mun_hir::CallableDef::Function(def)) => {
                    if self.module_group.should_runtime_link_fn(self.db, def) {
                        let fn_module = def.module(self.db);
                        if !def.is_extern(self.db) && !self.module_group.contains(fn_module) {
                            self.referenced_modules.insert(fn_module);
                        }
                        self.collect_fn_def(def);
                    }
                }
                Some(mun_hir::CallableDef::Struct(_)) => (),
                None => panic!("expected a callable expression"),
            }
        }

        // Recurse further
        expr.walk_child_exprs(|expr_id| self.collect_expr(expr_id, body, infer));
    }

    /// Collects function call expression from the given expression.
    #[allow(clippy::map_entry)]
    pub fn collect_fn_def(&mut self, function: mun_hir::Function) {
        self.ensure_table_ref();

        // If the function is not yet contained in the table, add it
        if !self.function_to_idx.contains_key(&function) {
            let name = function.full_name(self.db);
            let hir_type = function.ty(self.db);
            let sig = hir_type.callable_sig(self.db).unwrap();
            let ir_type = self.hir_types.get_function_type(function);
            let arg_types = sig
                .params()
                .iter()
                .map(|arg| self.hir_types.type_id(arg))
                .collect();
            let ret_type = if sig.ret().is_empty() {
                <()>::type_id().clone()
            } else {
                self.hir_types.type_id(sig.ret())
            };

            let prototype = FunctionPrototype {
                name,
                arg_types,
                ret_type,
            };
            let index = self.entries.len();
            self.entries.push(TypedDispatchableFunction {
                function: DispatchableFunction {
                    prototype: prototype.clone(),
                    mun_hir: Some(function),
                },
                ir_type,
            });
            self.prototype_to_idx.insert(prototype, index);
            self.function_to_idx.insert(function, index);
        }
    }

    /// Collect all the call expressions from the specified body with the given
    /// type inference result.
    pub fn collect_body(&mut self, body: &Arc<Body>, infer: &InferenceResult) {
        self.collect_expr(body.body_expr(), body, infer);
    }

    /// Builds the final `DispatchTable` with all *called* functions from within
    /// the module
    ///
    /// # Parameters
    ///
    /// * **functions**: Mapping of *defined* Mun functions to their respective
    ///   IR values.
    ///
    /// Returns the `DispatchTable` and a set of dependencies for the module.
    pub fn build(self) -> (DispatchTable<'ink>, FxHashSet<mun_hir::Module>) {
        // Construct the table body from all the entries in the dispatch table
        let table_body: Vec<BasicTypeEnum<'ink>> = self
            .entries
            .iter()
            .map(|f| f.ir_type.ptr_type(inkwell::AddressSpace::default()).into())
            .collect();

        // We can fill in the DispatchTable body, i.e: struct DispatchTable { <this
        // part> };
        self.table_type.set_body(&table_body, false);

        // Create a default initializer for function that are already known
        if let Some(table_ref) = self.table_ref {
            let values: Vec<BasicValueEnum<'ink>> = self
                .entries
                .iter()
                .enumerate()
                // Maps over all HIR functions
                .map(|(i, entry)| {
                    let function_type = table_body[i].into_pointer_type();
                    // Find the associated IR function if it exists
                    match entry.function.mun_hir {
                        // Case external function: Convert to typed null for the given function
                        None => function_type.const_null(),
                        // Case external function, or function from another module
                        Some(f) => {
                            if f.is_extern(self.db)
                                || !self.module_group.contains(f.module(self.db))
                            {
                                // If the function is externally defined (i.e. it's an `extern`
                                // function or it's defined in another module) don't initialize.
                                function_type.const_null()
                            } else {
                                // Otherwise generate a function prototype
                                function::gen_prototype(self.db, self.hir_types, f, self.module)
                                    .as_global_value()
                                    .as_pointer_value()
                            }
                        }
                    }
                    .into()
                })
                .collect();
            // Set the initialize for the global value
            table_ref.set_initializer(&self.table_type.const_named_struct(&values));
        }

        let table_type = self.table_ref.map(|_| self.table_type);

        (
            DispatchTable {
                context: self.context,
                target: self.target_data,
                function_to_idx: self.function_to_idx,
                prototype_to_idx: self.prototype_to_idx,
                table_ref: self.table_ref,
                table_type,
                entries: self
                    .entries
                    .into_iter()
                    .map(|entry| entry.function)
                    .collect(),
            },
            self.referenced_modules,
        )
    }
}
