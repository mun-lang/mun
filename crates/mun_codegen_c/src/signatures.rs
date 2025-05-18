use crate::CCodegenDatabase;
use c_codegen::{function, identifier};
use c_codegen::r#type::Function;
use crate::identifier::generate_function_name;

/// Returns the identifier used in the C code for the given function.
///
/// This function always returns a unique identifier.
pub fn function_identifier(
    db: &dyn CCodegenDatabase,
    fun: mun_hir::Function,
) -> identifier::Identifier {
    let function_name = fun.full_name(db.upcast());
    generate_function_name(&function_name)
}

/// Generates the function signature for the given function.
pub fn function_signature(
    db: &dyn CCodegenDatabase,
    fun: mun_hir::Function,
) -> function::Declaration {
    let return_ty = fun.ret_type(db.upcast());
    let arg_tys = fun.params(db.upcast());

    function::Declaration {
        is_static: false,
        name: db.function_identifier(fun),
        ty: Function {
            parameters: arg_tys.into_iter().map(|param| {
                let arg_ty = crate::ty::generate(param.ty());
                function::FunctionParameter {
                    ty: arg_ty,
                    name: None, // Don't generate a name for arguments in the signature
                }
            }).collect(),
            return_ty: crate::ty::generate(&return_ty),
        },
    }
}
