use c_codegen::{identifier, operator::ArraySubscript, r#type::{structure::Struct, InitializerList}, variable, Identifier, Value, Variable};
use mun_codegen::DispatchTable;
use mun_hir::HirDatabase;

const GLOBAL_DISPATCH_TABLE_NAME: &str = "g_dispatchTable";

/// Generate the initialization of the global dispatch table. This is equivalent to:
/// ```c
/// struct DispatchTable
/// ```
pub fn generate_initialization(
    dispatch_table: &DispatchTable,
    db: &dyn HirDatabase,
) -> Result<Variable, identifier::Error> {

    let initializer_list = dispatch_table.entries().iter().enumerate().map(|(index, function)| {
        if let Some(function) = function.mun_hir {

        } else {
            Value::Pointer { address: 0, base_type: Type }
        }
    })

    let declaration = variable::Declaration {
        storage_class: None,
        ty: Struct::Definition { name: None, member_groups: todo!() },
        variables: vec![(Identifier::new(GLOBAL_DISPATCH_TABLE_NAME)?,
            Some(todo!())
        )],
    };
    

    let initializer_list = InitializerList::Ordered(todo!());
}

/// Generate a function lookup through the `DispatchTable`, equivalent to
/// something along the lines of: `dispatchTable[i]`, where i is the
/// index of the function and `dispatchTable` is a struct
pub fn generate_function_lookup(
    dispatch_table: &DispatchTable,
    db: &dyn HirDatabase,
    function: mun_hir::Function,
) -> Result<ArraySubscript, identifier::Error> {
    // Get the index of the function
    let index = dispatch_table
        .index_by_function(function)
        .expect("unknown function");

    Ok(ArraySubscript {
        array: Variable::new(GLOBAL_DISPATCH_TABLE_NAME)?,
        index: Value::Size { value: index },
    })
}

/// Generates a function lookup through the `DispatchTable`, equivalent to
/// something along the lines of: `dispatchTable[i]`, where i is the
/// index of the intrinsic and `dispatchTable` is a struct
pub fn generate_intrinsic_lookup(
    dispatch_table: &DispatchTable,
    intrinsic: &impl Intrinsic,
) -> Result<ArraySubscript, identifier::Error> {
    let prototype = intrinsic.prototype();

    // Get the index of the intrinsic
    let index = dispatch_table
        .index_by_intrinsic(intrinsic)
        .expect("unknown intrinsic");

    Ok(ArraySubscript {
        array: Variable::new(GLOBAL_DISPATCH_TABLE_NAME)?,
        index: Value::Size { value: index },
    })
}
