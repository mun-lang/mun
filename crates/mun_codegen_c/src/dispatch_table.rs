use c_codegen::{
    identifier,
    operator::{ArraySubscript, PrefixOperator, PrefixOperatorKind},
    r#type::{member::Member, structure::Struct, InitializerList},
    variable, Expression, Identifier, Value, Variable,
};
use itertools::Either;
use mun_codegen::{DispatchTable, Intrinsic, ModuleGroup};
use mun_hir::HirDatabase;

use crate::{function, identifier::generate_function_name};

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
) -> variable::Declaration {
    let (members, values) = dispatch_table
        .entries()
        .iter()
        .map(|function| {
            let ty = match &function.mun_hir {
                Either::Left(function) => function::generate_pointer_type(
                    function.params(db).iter().map(mun_hir::Param::ty),
                    &function.ret_type(db),
                ),
                Either::Right(fn_sig) => {
                    function::generate_pointer_type(fn_sig.params().iter(), fn_sig.ret())
                }
            };

            let member = Member {
                ty,
                name: generate_function_name(&function.prototype.name),
                bit_field_size: None,
            };

            let expression: Expression = if let Either::Left(function) = function.mun_hir {
                // If the function is externally defined (i.e. it's an `extern`
                // function or it's defined in another module) don't initialize.
                if function.is_extern(db) || !module_group.contains(function.module(db)) {
                    Value::Pointer { address: 0 }.into()
                } else {
                    let function_name = function.name(db);

                    PrefixOperator {
                        operand: generate_function_name(&function_name.to_string()).into(),
                        operator: PrefixOperatorKind::Address,
                    }
                    .into()
                }
            } else {
                Value::Pointer { address: 0 }.into()
            };

            (member, expression)
        })
        .collect::<(Vec<Member>, Vec<Expression>)>();

    variable::Declaration {
        storage_class: None,
        ty: Struct::Definition {
            name: None,
            members,
        }
        .into(),
        identifier: Identifier::new(GLOBAL_DISPATCH_TABLE_NAME).expect("Invalid identifier"),
        initializer: Some(InitializerList::Ordered(values).into()),
    }
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
