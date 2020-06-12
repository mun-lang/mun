use crate::intrinsics::Intrinsic;
use crate::ir::function;
use crate::type_info::TypeInfo;
use crate::{CodeGenParams, IrDatabase};
use hir::{Body, Expr, ExprId, InferenceResult};
use inkwell::{
    context::Context,
    module::Module,
    targets::TargetData,
    types::{BasicTypeEnum, FunctionType},
    values::{BasicValueEnum, PointerValue},
};
use std::collections::{BTreeMap, HashMap};
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
    // The LLVM context in which all LLVM types live
    context: Arc<Context>,
    // The target for which to create the dispatch table
    target: Arc<TargetData>,
    // This contains the function that map to the DispatchTable struct fields
    function_to_idx: HashMap<hir::Function, usize>,
    // Prototype to function index
    prototype_to_idx: HashMap<FunctionPrototype, usize>,
    // This contains an ordered list of all the function in the dispatch table
    entries: Vec<DispatchableFunction>,
    // Contains a reference to the global value containing the DispatchTable
    table_ref: Option<inkwell::values::GlobalValue>,
    //
    table_type: Option<inkwell::types::StructType>,
}

/// A `FunctionPrototype` defines a unique signature that can be added to the dispatch table.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
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
    /// Returns whether the `DispatchTable` contains the specified `function`.
    pub fn contains(&self, function: hir::Function) -> bool {
        self.function_to_idx.contains_key(&function)
    }

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
        table_ref: Option<inkwell::values::GlobalValue>,
        builder: &inkwell::builder::Builder,
        function: hir::Function,
    ) -> PointerValue {
        let function_name = function.name(db).to_string();

        // Get the index of the function
        let index = *self
            .function_to_idx
            .get(&function)
            .expect("unknown function");

        self.gen_function_lookup_by_index(table_ref, builder, &function_name, index)
    }

    /// Generates a function lookup through the DispatchTable, equivalent to something along the
    /// lines of: `dispatchTable[i]`, where i is the index of the intrinsic and `dispatchTable` is a
    /// struct
    pub fn gen_intrinsic_lookup(
        &self,
        table_ref: Option<inkwell::values::GlobalValue>,
        builder: &inkwell::builder::Builder,
        intrinsic: &impl Intrinsic,
    ) -> PointerValue {
        let prototype = intrinsic.prototype(self.context.as_ref(), self.target.as_ref());

        // Get the index of the intrinsic
        let index = *self
            .prototype_to_idx
            .get(&prototype)
            .expect("unknown function");

        self.gen_function_lookup_by_index(table_ref, builder, &prototype.name, index)
    }

    /// Generates a function lookup through the DispatchTable, equivalent to something along the
    /// lines of: `dispatchTable[i]`, where i is the index and `dispatchTable` is a struct
    fn gen_function_lookup_by_index(
        &self,
        table_ref: Option<inkwell::values::GlobalValue>,
        builder: &inkwell::builder::Builder,
        function_name: &str,
        index: usize,
    ) -> PointerValue {
        // Get the internal table reference
        let table_ref = table_ref.expect("no dispatch table defined");

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

    /// Returns the IR type of the dispatch table's global value, if it exists.
    pub fn ty(&self) -> Option<inkwell::types::StructType> {
        self.table_type
    }
}

/// A struct that can be used to build the dispatch table from HIR.
pub(crate) struct DispatchTableBuilder<'a, D: IrDatabase> {
    db: &'a D,
    // The LLVM context in which all LLVM types live
    context: Arc<Context>,
    // The module in which all values live
    module: &'a Module,
    // The target for which to create the dispatch table
    target: Arc<TargetData>,
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
    pub fn new(
        db: &'a D,
        module: &'a Module,
        intrinsics: &BTreeMap<FunctionPrototype, FunctionType>,
    ) -> Self {
        let mut table = DispatchTableBuilder {
            db,
            context: db.context(),
            module,
            target: db.target_data(),
            function_to_idx: Default::default(),
            prototype_to_idx: Default::default(),
            entries: Default::default(),
            table_ref: None,
            table_type: module.get_context().opaque_struct_type("DispatchTable"),
        };

        if !intrinsics.is_empty() {
            table.ensure_table_ref();

            // Use a `BTreeMap` to guarantee deterministically ordered output
            for (prototype, ir_type) in intrinsics.iter() {
                let index = table.entries.len();
                table.entries.push(TypedDispatchableFunction {
                    function: DispatchableFunction {
                        prototype: prototype.clone(),
                        hir: None,
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
                Some(hir::CallableDef::Struct(_)) => (),
                None => panic!("expected a callable expression"),
            }
        }

        // Recurse further
        expr.walk_child_exprs(|expr_id| self.collect_expr(expr_id, body, infer));
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
            let ir_type = self
                .db
                .type_ir(
                    hir_type,
                    CodeGenParams {
                        make_marshallable: false,
                    },
                )
                .into_function_type();
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
            let index = self.entries.len();
            self.entries.push(TypedDispatchableFunction {
                function: DispatchableFunction {
                    prototype: prototype.clone(),
                    hir: Some(function),
                },
                ir_type,
            });
            self.prototype_to_idx.insert(prototype, index);
            self.function_to_idx.insert(function, index);

            // Recurse further
            let fn_body = function.body(self.db);
            self.collect_expr(
                fn_body.body_expr(),
                &fn_body,
                function.infer(self.db).as_ref(),
            );
        }
    }

    /// Collect all the call expressions from the specified body with the given type inference
    /// result.
    pub fn collect_body(&mut self, body: &Arc<Body>, infer: &InferenceResult) {
        self.collect_expr(body.body_expr(), body, infer);
    }

    /// Builds the final DispatchTable with all *called* functions from within the module
    /// # Parameters
    /// * **functions**: Mapping of *defined* Mun functions to their respective IR values.
    pub fn build(self) -> DispatchTable {
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
                    match entry.function.hir {
                        // Case external function: Convert to typed null for the given function
                        None => function_type.const_null(),
                        Some(f) if f.is_extern(self.db) => function_type.const_null(),
                        // Case mun function: Get the function location as the initializer
                        Some(f) => function::gen_signature(
                            self.db,
                            f,
                            self.module,
                            CodeGenParams {
                                make_marshallable: false,
                            },
                        )
                        .as_global_value()
                        .as_pointer_value(),
                    }
                    .into()
                })
                .collect();
            // Set the initialize for the global value
            table_ref.set_initializer(&self.table_type.const_named_struct(&values));
        }

        let table_type = self.table_ref.map(|_| self.table_type);

        DispatchTable {
            context: self.context,
            target: self.target,
            function_to_idx: self.function_to_idx,
            prototype_to_idx: self.prototype_to_idx,
            table_ref: self.table_ref,
            table_type,
            entries: self
                .entries
                .into_iter()
                .map(|entry| entry.function)
                .collect(),
        }
    }
}
