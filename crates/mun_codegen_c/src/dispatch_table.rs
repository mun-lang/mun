use c_codegen::{
    identifier,
    operator::{ArraySubscript, PrefixOperator, PrefixOperatorKind},
    r#type::{
        member::{self, Member},
        structure::Struct,
        InitializerList,
    },
    variable, Expression, Identifier, Value, Variable,
};
use mun_codegen::{DispatchTable, Intrinsic, ModuleGroup};
use mun_hir::HirDatabase;

use crate::function;

const GLOBAL_DISPATCH_TABLE_NAME: &str = "g_dispatchTable";

/// Generate the initialization of the global dispatch table. This is equivalent to:
/// ```c
/// struct DispatchTable {
///     void (*fn0)(void);
///     void (*fn1)(int);
/// } g_dispatchTable = { &fn0, &fn1 };
/// ```
pub fn generate_initialization(
    module_group: &ModuleGroup,
    dispatch_table: &DispatchTable,
    db: &dyn HirDatabase,
) -> c_codegen::Result<variable::Declaration> {
    let (member_groups, values) = dispatch_table
        .entries()
        .iter()
        .map(|function| {
            let ty = function::generate_pointer_type(&function.prototype);
            let group = member::Group {
                ty,
                members: vec![Member {
                    name: Identifier::new(&function.prototype.name)?,
                    bit_field_size: None,
                }]
                .try_into()?,
            };

            let expression: Expression = if let Some(function) = function.mun_hir {
                // If the function is externally defined (i.e. it's an `extern`
                // function or it's defined in another module) don't initialize.
                if function.is_extern(db) || !module_group.contains(function.module(db)) {
                    Value::Pointer { address: 0 }.into()
                } else {
                    let function_name = function.name(db);

                    PrefixOperator {
                        operand: Variable::new(function_name.to_string())?.into(),
                        operator: PrefixOperatorKind::Address,
                    }
                    .into()
                }
            } else {
                Value::Pointer { address: 0 }.into()
            };

            Ok((group, expression))
        })
        .collect::<c_codegen::Result<(Vec<member::Group>, Vec<Expression>)>>()?;

    let declaration = variable::Declaration {
        storage_class: None,
        ty: Struct::Definition {
            name: None,
            member_groups: member_groups.try_into()?,
        }
        .into(),
        variables: vec![(
            Identifier::new(GLOBAL_DISPATCH_TABLE_NAME)?,
            Some(InitializerList::Ordered(values).into()),
        )]
        .try_into()?,
    };

    Ok(declaration)
}

/// Generate a function lookup through the `DispatchTable`, equivalent to
/// something along the lines of: `dispatchTable[i]`, where i is the
/// index of the function and `dispatchTable` is a struct
pub fn generate_function_lookup(
    dispatch_table: &DispatchTable,
    function: mun_hir::Function,
) -> Result<ArraySubscript, identifier::Error> {
    // Get the index of the function
    let index = dispatch_table
        .index_by_function(function)
        .expect("unknown function");

    Ok(ArraySubscript {
        array: Variable::new(GLOBAL_DISPATCH_TABLE_NAME)?.into(),
        index: Value::Size { value: index }.into(),
    })
}

/// Generates a function lookup through the `DispatchTable`, equivalent to
/// something along the lines of: `dispatchTable[i]`, where i is the
/// index of the intrinsic and `dispatchTable` is a struct
pub fn generate_intrinsic_lookup(
    dispatch_table: &DispatchTable,
    intrinsic: &impl Intrinsic,
) -> Result<ArraySubscript, identifier::Error> {
    // Get the index of the intrinsic
    let index = dispatch_table
        .index_by_intrinsic(intrinsic)
        .expect("unknown intrinsic");

    Ok(ArraySubscript {
        array: Variable::new(GLOBAL_DISPATCH_TABLE_NAME)?.into(),
        index: Value::Size { value: index }.into(),
    })
}
