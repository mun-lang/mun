use c_codegen::{r#type::Pointer, Type};
use mun_codegen::FunctionPrototype;

pub fn generate_pointer_type(prototype: &FunctionPrototype) -> Type {
    // TODO: C codegen doesn't include function pointer
    Type::Pointer(Pointer {
        pointer_ty: Box::new(Type::Void),
        is_const: false,
    })
}
