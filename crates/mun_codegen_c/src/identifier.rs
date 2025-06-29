use c_codegen::Identifier;

pub(crate) fn full_name_to_identifier(name: &str) -> Identifier {
    Identifier::new(name.replace("::", "_"))
        .unwrap_or_else(|_| panic!("Invalid identifier: {name}"))
}
