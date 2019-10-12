use crate::values::FunctionValue;
use crate::IrDatabase;
use inkwell::module::Module;
use inkwell::types::{PointerType, BasicTypeEnum};
use inkwell::values::{BasicValueEnum, PointerValue};
use mun_hir as hir;
use std::collections::HashMap;

pub(crate) struct DispatchTableBuilder<'a, D: IrDatabase> {
    db: &'a D,
    module: &'a Module,
    // This contains the function that map to the DispatchTable struct fields
    function_to_idx: HashMap<hir::Function, usize>,
    // These are *all* called functions in the modules
    entries: Vec<hir::Function>,
    // Contains a reference to the global value containing the DispatchTable
    table_ref: Option<inkwell::values::GlobalValue>,
    // This is the actual DispatchTable type
    table_type: inkwell::types::StructType,
    // This is the table body
    table_body: Vec<BasicTypeEnum>,
}

impl<'a, D: IrDatabase> DispatchTableBuilder<'a, D> {
    /// Create a new builder that can generate a dispatch function
    pub fn new(db: &'a D, module: &'a Module) -> Self {
        DispatchTableBuilder {
            db,
            module,
            function_to_idx: Default::default(),
            entries: Default::default(),
            table_ref: None,
            table_type: module.get_context().opaque_struct_type("DispatchTable"),
            table_body: Default::default()
        }
    }

    /// Get the reference to the DispatchTable, this function creates the table if it does not exist
    pub fn get_table_ref(&mut self) -> &inkwell::values::GlobalValue {
        if self.table_ref.is_none() {
            self.table_ref = Some(
                self.module
                    .add_global(self.table_type, None, "dispatchTable"),
            )
        }
        self.table_ref.as_ref().unwrap()
    }

    /// Generate a function lookup through the DispatchTable, equivalent to something along the
    /// lines of: `dispatchTable[i]`, where i is the index of the function and `dispatchTable` is a
    /// struct
    pub fn gen_function_lookup(
        &mut self,
        builder: &inkwell::builder::Builder,
        function: &hir::Function,
    ) -> PointerValue {
        let function_name = function.name(self.db).to_string();

        // Get the index of the function or add this
        let index = match self.function_to_idx.get(function) {
            None => {
                // Insert into function map
                self.entries.push(*function);
                self.function_to_idx.insert(*function, self.function_to_idx.len());

                // Push the associated IR function value type in the table_body field
                // So that we can fill the DispatchTable struct intermediately
                self.table_body.push(self.db
                    // This returns the associated IR type declaration
                    .type_ir(function.ty(self.db))
                    .into_function_type()
                    // This converts it into a function pointer type
                    .ptr_type(inkwell::AddressSpace::Generic)
                    .into());

                // We can fill in the DispatchTable body, i.e: struct DispatchTable { <this part> };
                eprintln!("check");
                self.table_type.set_body(&self.table_body, false);

                self.function_to_idx.len() - 1
            }
            Some(idx) => *idx
        };

        // Get the internal table reference
        let table_ref = self.get_table_ref();
        // Create an expression that finds the associated field in the table and returns this as a pointer access
        let ptr_to_function_ptr = unsafe {
            builder.build_struct_gep(
                table_ref.as_pointer_value(),
                index as u32,
                &format!("{0}_ptr_ptr", function_name)
            )
        };
        builder.build_load(ptr_to_function_ptr, &format!("{0}_ptr", function_name)).into_pointer_value()
    }

    /// This creates the final DispatchTable with all *called* function from within the module
    /// # Parameters
    /// * **functions**: This is mapping of *defined* mun-functions mapped to the respective IR values
    pub fn finalize(
        mut self,
        functions: &HashMap<mun_hir::Function, FunctionValue>,
    ) -> (Option<inkwell::values::GlobalValue>, Vec<hir::Function>) {

        // Create a default initializer for function that are already known
        if let Some(table_ref) = self.table_ref {
            let values: Vec<BasicValueEnum> = self
                .entries
                .iter()
                .enumerate()
                // Maps over all HIR functions
                .map(|(i, f)| {
                    let function_type = self.table_body[i].into_pointer_type();
                    // Find the associated IR function if it exists
                    match functions.get(f) {
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
        (self.table_ref, self.entries)
    }
}
