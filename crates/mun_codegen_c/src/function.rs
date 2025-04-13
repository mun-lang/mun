use c_codegen::{
    function::FunctionParameter,
    r#type::{Function, Pointer},
    ConcreteType,
};

use crate::ty;

pub fn generate_pointer_type<'ty>(
    parameters: impl Iterator<Item = &'ty mun_hir::Ty>,
    return_ty: &mun_hir::Ty,
) -> ConcreteType {
    ConcreteType::Pointer(Box::new(Pointer {
        pointer_ty: Function {
            parameters: parameters
                .map(|parameter| {
                    let ty = ty::generate(parameter);

                    FunctionParameter { ty, name: None }
                })
                .collect(),
            return_ty: ty::generate(return_ty),
        }
        .into(),
        is_const: false,
    }))
}
