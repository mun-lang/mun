use std::sync::Arc;

use c_codegen::{
    operator::ArraySubscript,
    r#type::{Array, InitializerList, Pointer},
    variable, ConcreteType, Expression, Identifier, Value,
};
use mun_codegen::{TypeId, TypeTable};

const GLOBAL_TYPE_TABLE_NAME: &str = "global_type_lookup_table";

/// Generate the initialization of the global type table. This is equivalent to:
/// ```c
/// void *const global_type_lookup_table[] = {
///     0x0,
///     0x0,
///     ...
/// };
/// ```
pub fn maybe_generate_initialization(type_table: &TypeTable) -> Option<variable::Declaration> {
    if type_table.is_empty() {
        return None;
    }

    let entries = type_table
        .entries()
        .iter()
        .map(|_| Value::Pointer { address: 0x0 }.into())
        .collect::<Vec<Expression>>();

    let identifier = Identifier::new(GLOBAL_TYPE_TABLE_NAME)
        .unwrap_or_else(|_| panic!("Invalid identifier: {GLOBAL_TYPE_TABLE_NAME}"));

    Some(variable::Declaration {
        storage_class: None,
        ty: Array {
            element_type: Box::new(
                Pointer {
                    pointer_ty: ConcreteType::Void.into(),
                    is_const: true,
                }
                .into(),
            ),
            size: Some(entries.len()),
        }
        .into(),
        identifier,
        initializer: Some(InitializerList::Ordered(entries).into()),
    })
}

/// Generates a type lookup through the [`TypeTable`], equivalent to
/// something along the lines of: `type_table[i]`, where `i` is the
/// index of the type and `type_table` is an array of `TypeInfo`
/// pointers.
pub fn generate_type_lookup(type_table: &TypeTable, type_id: &Arc<TypeId>) -> ArraySubscript {
    let global = Identifier::new(GLOBAL_TYPE_TABLE_NAME)
        .unwrap_or_else(|_| panic!("Invalid identifier: {GLOBAL_TYPE_TABLE_NAME}"));

    let index = type_table
        .index_of_type(type_id)
        .unwrap_or_else(|| panic!("Unknown type: {type_id}"));

    ArraySubscript {
        array: global.into(),
        index: Value::Size { value: index }.into(),
    }
}
