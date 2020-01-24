use crate::intrinsics;
use crate::values::FunctionValue;
use crate::IrDatabase;
use inkwell::module::Module;
use inkwell::types::{BasicTypeEnum, FunctionType};
use inkwell::values::{BasicValueEnum, PointerValue};

use crate::intrinsics::Intrinsic;
use crate::type_info::TypeInfo;
use hir::{Body, Expr, ExprId, InferenceResult};
use std::collections::HashMap;
use std::sync::Arc;

/// A dispatch table in IR is a struct that contains pointers to all functions that are called from
/// code. In C terms it looks something like this:
/// ```c
/// struct DispatchTable {
///     int(*foo)(int, int);
///     // .. etc
/// } dispatchTable;
/// ```
///
/// The dispatch table is used to add a patchable indirection when calling a function from IR. The
/// DispatchTable is exposed to the Runtime which fills the structure with valid pointers to
/// functions. This basically enables all hot reloading within Mun.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DispatchTable {
    // This contains the function that map to the DispatchTable struct fields
    function_to_idx: HashMap<hir::Function, usize>,
    // Prototype to function index
    prototype_to_idx: HashMap<FunctionPrototype, usize>,
    // This contains an ordered list of all the function in the dispatch table
    entries: Vec<DispatchableFunction>,
    // Contains a reference to the global value containing the DispatchTable
    table_ref: Option<inkwell::values::GlobalValue>,
}

/// A `FunctionPrototype` defines a unique signature that can be added to the dispatch table.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct FunctionPrototype {
    pub name: String,
    pub arg_types: Vec<TypeInfo>,
    pub ret_type: Option<TypeInfo>,
}

/// A `DispatchableFunction` is an entry in the dispatch table that may or may not be pointing to an
/// existing hir function.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DispatchableFunction {
    pub prototype: FunctionPrototype,
    pub hir: Option<hir::Function>,
}

impl DispatchTable {
    /// Returns a slice containing all the functions in the dispatch table.
    pub fn entries(&self) -> &[DispatchableFunction] {
        &self.entries
    }

    /// Generate a function lookup through the DispatchTable, equivalent to something along the
    /// lines of: `dispatchTable[i]`, where i is the index of the function and `dispatchTable` is a
    /// struct
    pub fn gen_function_lookup<D: IrDatabase>(
        &self,
        db: &D,
        builder: &inkwell::builder::Builder,
        function: hir::Function,
    ) -> PointerValue {
        let function_name = function.name(db).to_string();

        // Get the index of the function
        let index = *self
            .function_to_idx
            .get(&function)
            .expect("unknown function");

        self.gen_function_lookup_by_index(builder, &function_name, index)
    }

    /// Generates a function lookup through the DispatchTable, equivalent to something alone the
    /// lines of: `dispatchTable[i]`, where i is the index of the intrinsic and `dispatchTable` is a
    /// struct
    pub fn gen_intrinsic_lookup(
        &self,
        builder: &inkwell::builder::Builder,
        intrinsic: &impl Intrinsic,
    ) -> PointerValue {
        let prototype = intrinsic.prototype();

        // Get the index of the intrinsic
        let index = *self
            .prototype_to_idx
            .get(&prototype)
            .expect("unknown function");

        self.gen_function_lookup_by_index(builder, &prototype.name, index)
    }

    /// Generates a function lookup through the DispatchTable, equivalent to something alone the
    /// lines of: `dispatchTable[i]`, where i is the index and `dispatchTable` is a struct
    fn gen_function_lookup_by_index(
        &self,
        builder: &inkwell::builder::Builder,
        function_name: &str,
        index: usize,
    ) -> PointerValue {
        // Get the internal table reference
        let table_ref = self.table_ref.expect("no dispatch table defined");

        // Create an expression that finds the associated field in the table and returns this as a pointer access
        let ptr_to_function_ptr = unsafe {
            builder.build_struct_gep(
                table_ref.as_pointer_value(),
                index as u32,
                &format!("{0}_ptr_ptr", function_name),
            )
        };
        builder
            .build_load(ptr_to_function_ptr, &format!("{0}_ptr", function_name))
            .into_pointer_value()
    }

    /// Returns the value that represents the dispatch table in IR or `None` if no table was
    /// generated.
    pub fn global_value(&self) -> Option<&inkwell::values::GlobalValue> {
        self.table_ref.as_ref()
    }
}

/// A struct that can be used to build the dispatch table from HIR.
pub(crate) struct DispatchTableBuilder<'a, D: IrDatabase> {
    db: &'a D,
    module: &'a Module,
    // This contains the functions that map to the DispatchTable struct fields
    function_to_idx: HashMap<hir::Function, usize>,
    // Prototype to function index
    prototype_to_idx: HashMap<FunctionPrototype, usize>,
    // These are *all* called functions in the modules
    entries: Vec<TypedDispatchableFunction>,
    // Contains a reference to the global value containing the DispatchTable
    table_ref: Option<inkwell::values::GlobalValue>,
    // This is the actual DispatchTable type
    table_type: inkwell::types::StructType,
}

struct TypedDispatchableFunction {
    function: DispatchableFunction,
    ir_type: FunctionType,
}

impl<'a, D: IrDatabase> DispatchTableBuilder<'a, D> {
    /// Creates a new builder that can generate a dispatch function.
    pub fn new(db: &'a D, module: &'a Module) -> Self {
        DispatchTableBuilder {
            db,
            module,
            function_to_idx: Default::default(),
            prototype_to_idx: Default::default(),
            entries: Default::default(),
            table_ref: None,
            table_type: module.get_context().opaque_struct_type("DispatchTable"),
        }
    }

    /// Creates the global dispatch table in the module if it does not exist.
    fn ensure_table_ref(&mut self) {
        if self.table_ref.is_none() {
            self.table_ref = Some(
                self.module
                    .add_global(self.table_type, None, "dispatchTable"),
            )
        }
    }

    /// Collects call expression from the given expression and sub expressions.
    fn collect_expr(&mut self, expr_id: ExprId, body: &Arc<Body>, infer: &InferenceResult) {
        let expr = &body[expr_id];

        // If this expression is a call, store it in the dispatch table
        if let Expr::Call { callee, .. } = expr {
            match infer[*callee].as_callable_def() {
                Some(hir::CallableDef::Function(def)) => self.collect_fn_def(def),
                Some(hir::CallableDef::Struct(s)) => {
                    if s.data(self.db).memory_kind == hir::StructMemoryKind::GC {
                        self.collect_intrinsic(&intrinsics::malloc)
                    }
                }
                None => panic!("expected a callable expression"),
            }
        }

        if let Expr::RecordLit { .. } = expr {
            let struct_ty = infer[expr_id].clone();
            let hir_struct = struct_ty.as_struct().unwrap(); // Can only really get here if the type is a struct
            if hir_struct.data(self.db).memory_kind == hir::StructMemoryKind::GC {
                self.collect_intrinsic(&intrinsics::malloc)
            }
        }

        if let Expr::Path(path) = expr {
            let resolver = hir::resolver_for_expr(body.clone(), self.db, expr_id);
            let resolution = resolver
                .resolve_path_without_assoc_items(self.db, path)
                .take_values()
                .expect("unknown path");

            if let hir::Resolution::Def(hir::ModuleDef::Struct(s)) = resolution {
                if s.data(self.db).memory_kind == hir::StructMemoryKind::GC {
                    self.collect_intrinsic(&intrinsics::malloc)
                }
            }
        }

        // Recurse further
        expr.walk_child_exprs(|expr_id| self.collect_expr(expr_id, body, infer))
    }

    /// Collects function call expression from the given expression.
    #[allow(clippy::map_entry)]
    fn collect_fn_def(&mut self, function: hir::Function) {
        self.ensure_table_ref();

        // If the function is not yet contained in the table, add it
        if !self.function_to_idx.contains_key(&function) {
            let name = function.name(self.db).to_string();
            let hir_type = function.ty(self.db);
            let sig = hir_type.callable_sig(self.db).unwrap();
            let ir_type = self.db.type_ir(hir_type).into_function_type();
            let arg_types = sig
                .params()
                .iter()
                .map(|arg| self.db.type_info(arg.clone()))
                .collect();
            let ret_type = if !sig.ret().is_empty() {
                Some(self.db.type_info(sig.ret().clone()))
            } else {
                None
            };

            let prototype = FunctionPrototype {
                name,
                arg_types,
                ret_type,
            };
            self.entries.push(TypedDispatchableFunction {
                function: DispatchableFunction {
                    prototype: prototype.clone(),
                    hir: Some(function),
                },
                ir_type,
            });
            self.prototype_to_idx
                .insert(prototype, self.function_to_idx.len());
            self.function_to_idx
                .insert(function, self.function_to_idx.len());
        }
    }

    /// Collects a call to an intrinsic function.
    #[allow(clippy::map_entry)]
    fn collect_intrinsic(&mut self, intrinsic: &impl Intrinsic) {
        self.ensure_table_ref();

        // If the function is not yet contained in the table add it
        let prototype = intrinsic.prototype();
        if !self.prototype_to_idx.contains_key(&prototype) {
            self.entries.push(TypedDispatchableFunction {
                function: DispatchableFunction {
                    prototype: prototype.clone(),
                    hir: None,
                },
                ir_type: intrinsic.ir_type(&self.module.get_context()),
            });

            self.prototype_to_idx
                .insert(prototype, self.function_to_idx.len());
        }
    }

    /// Collect all the call expressions from the specified body with the given type inference
    /// result.
    pub fn collect_body(&mut self, body: &Arc<Body>, infer: &InferenceResult) {
        self.collect_expr(body.body_expr(), body, infer);
    }

    /// This creates the final DispatchTable with all *called* functions from within the module
    /// # Parameters
    /// * **functions**: Mapping of *defined* Mun functions to their respective IR values.
    pub fn finalize(self, functions: &HashMap<hir::Function, FunctionValue>) -> DispatchTable {
        // Construct the table body from all the entries in the dispatch table
        let table_body: Vec<BasicTypeEnum> = self
            .entries
            .iter()
            .map(|f| f.ir_type.ptr_type(inkwell::AddressSpace::Generic).into())
            .collect();

        // We can fill in the DispatchTable body, i.e: struct DispatchTable { <this part> };
        self.table_type.set_body(&table_body, false);

        // Create a default initializer for function that are already known
        if let Some(table_ref) = self.table_ref {
            let values: Vec<BasicValueEnum> = self
                .entries
                .iter()
                .enumerate()
                // Maps over all HIR functions
                .map(|(i, entry)| {
                    let function_type = table_body[i].into_pointer_type();
                    // Find the associated IR function if it exists
                    match entry.function.hir.and_then(|f| functions.get(&f)) {
                        // Case external function: Convert to typed null for the given function
                        None => function_type.const_null(),
                        // Case mun function: Get the function location as the initializer
                        Some(function_value) => function_value.as_global_value().as_pointer_value(),
                    }
                    .into()
                })
                .collect();
            // Set the initialize for the global value
            table_ref.set_initializer(&self.table_type.const_named_struct(&values));
        }

        DispatchTable {
            function_to_idx: self.function_to_idx,
            prototype_to_idx: self.prototype_to_idx,
            table_ref: self.table_ref,
            entries: self
                .entries
                .into_iter()
                .map(|entry| entry.function)
                .collect(),
        }
    }
}
