use c_codegen::Identifier;

pub(crate) fn full_name_to_identifier(name: &str) -> Identifier {
    Identifier::new(name.replace("::", "_"))
        .unwrap_or_else(|_| panic!("Invalid identifier: {name}"))
}

pub(crate) fn field_name_to_identifier(name: &str) -> Identifier {
    let first_char = name.chars().next().unwrap();

    if first_char.is_ascii_digit() {
        Identifier::new(format!("_{name}"))
    } else {
        Identifier::new(name)
    }
    .unwrap_or_else(|_| panic!("Invalid field identifier: {name}"))
}
