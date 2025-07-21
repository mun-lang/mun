use crate::function::generate_parameters;
use crate::identifier::full_name_to_identifier;
use crate::CCodegenDatabase;
use c_codegen::r#type::Function;
use c_codegen::{function, identifier};

/// Returns the identifier used in the C code for the given function.
///
/// This function always returns a unique identifier.
pub fn function_identifier(
    db: &dyn CCodegenDatabase,
    fun: mun_hir::Function,
) -> identifier::Identifier {
    let function_name = fun.full_name(db.upcast());
    full_name_to_identifier(&function_name)
}

/// Generates the function signature for the given function.
pub fn function_signature(
    db: &dyn CCodegenDatabase,
    fun: mun_hir::Function,
) -> function::Declaration {
    let return_ty = fun.ret_type(db.upcast());

    // Don't generate a name for arguments in the signature
    let parameters = generate_parameters(db.upcast(), fun, false);

    function::Declaration {
        is_static: false,
        name: db.function_identifier(fun),
        ty: Function {
            parameters,
            return_ty: crate::ty::generate(db.upcast(), &return_ty),
        },
    }
}
