use c_codegen::Identifier;

pub(crate) fn generate_function_name(name: &str) -> Identifier {
    Identifier::new(name.replace("::", "_"))
        .unwrap_or_else(|_| panic!("Invalid identifier: {name}"))
}
