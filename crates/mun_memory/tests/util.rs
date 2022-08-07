#![allow(dead_code)]

#[macro_export]
macro_rules! fake_struct {
    ($type_table:expr, $struct_name:expr, $($field_name:expr => $field_ty:ident),+) => {{
        mun_memory::StructTypeBuilder::new(String::from($struct_name))
            $(
                .add_field(
                    String::from($field_name),
                    $type_table.find_type_info_by_name(format!("core::{}", stringify!($field_ty))).unwrap()
                )
            )+
             .finish()
    }};
}
