#![cfg_attr(tarpaulin, skip)]

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Lit, Meta, NestedMeta, Path};

/// This procedural macro implements the `AsValue` trait as well as several required other traits.
/// All of these traits enable creating an `inkwell::values::StructValue` from a generic struct, as
/// long as all fields of the struct also implement `AsValue`.
#[proc_macro_derive(AsValue, attributes(ir_name))]
pub fn as_value_derive(input: TokenStream) -> TokenStream {
    // Parse Phase
    let derive_input = parse_macro_input!(input as DeriveInput);
    let struct_data = match derive_input.data {
        Data::Struct(data) => data,
        Data::Union(_) => panic!("#[derive(AsValue)] is only defined for structs, not for unions!"),
        Data::Enum(_) => panic!("#[derive(AsValue)] is only defined for structs, not for enums!"),
    };

    // Parse the `[ir_name = ".."]` part
    let mut ir_name = String::new();
    for attr in derive_input
        .attrs
        .iter()
        .filter(|a| a.path.get_ident().map(|i| *i == "ir_name").unwrap_or(false))
        .map(Attribute::parse_meta)
        .filter_map(|x| x.ok())
    {
        if let Meta::NameValue(meta_name_value) = attr {
            match meta_name_value.lit {
                Lit::Str(lit_str) => {
                    ir_name = lit_str.value();
                }
                _ => {
                    panic!("ir_name must be a string");
                }
            };
        }
    }

    // Get the typename of the struct we're working with
    let ident = &derive_input.ident;

    // Generate a list of all field types
    let field_types_tuple = struct_data.fields.iter().map(|f| {
        let ty = &f.ty;
        quote! {
            #ty
        }
    });

    // Generate a list of where clauses that ensure that we can cast each field to an
    // `inkwell::types::BasicTypeEnum`
    let field_types = struct_data.fields.iter().map(|f| {
        let ty = &f.ty;
        quote! {
            Into::<inkwell::types::BasicTypeEnum>::into(<#ty>::get_ir_type(context))
        }
    });

    // Generate a list of where clauses that ensure that we can cast each field to an
    // `inkwell::values::BasicTypeValue`
    let field_types_values = struct_data.fields.iter().enumerate().map(|(idx, f)| {
        let name = f.ident.as_ref().map(|i| quote! { #i }).unwrap_or_else(|| quote! { #idx });
        quote! {
            crate::value::AsValueInto::<inkwell::values::BasicValueEnum>::as_value_into(&self. #name, context)
        }
    });

    // Generate Phase
    (quote! {
        impl crate::value::ConcreteValueType for #ident {
            type Value = inkwell::values::StructValue;
        }

        impl crate::value::SizedValueType for #ident {
            fn get_ir_type(context: &crate::value::IrTypeContext) -> inkwell::types::StructType {
                let key = (#ir_name, std::any::TypeId::of::<(#(#field_types_tuple),*)>());
                let struct_types = context.struct_types.upgradable_read();
                let value = match struct_types.get(&key) {
                    Some(value) => {
                        return *value;
                    }
                    None => {
                        let mut struct_types = parking_lot::RwLockUpgradableReadGuard::upgrade(struct_types);
                        let struct_ty = context.context.opaque_struct_type(key.0);
                        struct_types.insert(key, struct_ty);
                        struct_ty
                    }
                };
                value.set_body(&[
                    #(#field_types),*
                ], false);
                value
            }
        }

        impl crate::value::PointerValueType for #ident {
            fn get_ptr_type(context: &crate::value::IrTypeContext, address_space: Option<inkwell::AddressSpace>) -> inkwell::types::PointerType {
                Self::get_ir_type(context).ptr_type(address_space.unwrap_or(inkwell::AddressSpace::Generic))
            }
        }

        impl crate::value::AsValue<#ident> for #ident {
            fn as_value(&self, context: &crate::value::IrValueContext) -> crate::value::Value<Self> {
                let struct_type = Self::get_ir_type(context.type_context);
                crate::value::Value::from_raw(struct_type.const_named_struct(&[
                    #(#field_types_values),*
                ]))
            }
        }

        impl crate::value::AddressableType<#ident> for #ident {}
    }).into()
}

/// A procedural macro that implements the `TestIsAbiCompatible` trait for a struct. This
/// implementation enables testing for every field of a struct whether its abi type is compatible
/// with the current implementation.
#[proc_macro_derive(TestIsAbiCompatible, attributes(abi_type))]
pub fn is_abi_compatible_derive(input: TokenStream) -> TokenStream {
    // Parse Phase
    let derive_input = parse_macro_input!(input as DeriveInput);
    let struct_data = match derive_input.data {
        Data::Struct(data) => data,
        Data::Union(_) => {
            panic!("#[derive(IsAbiCompatible)] is only defined for structs, not for unions!")
        }
        Data::Enum(_) => {
            panic!("#[derive(IsAbiCompatible)] is only defined for structs, not for enums!")
        }
    };

    // Parse the [abi_type(...)] part
    let mut abi_type_name: Option<Path> = None;
    for attr in derive_input
        .attrs
        .iter()
        .filter(|a| {
            a.path
                .get_ident()
                .map(|i| *i == "abi_type")
                .unwrap_or(false)
        })
        .map(Attribute::parse_meta)
        .filter_map(|x| x.ok())
    {
        if let Meta::List(meta_list) = attr {
            if meta_list.nested.len() != 1 {
                panic!("expected abi_type to be a single path")
            } else if let NestedMeta::Meta(Meta::Path(p)) = meta_list.nested.first().unwrap() {
                abi_type_name = Some(p.clone());
            }
        } else {
            panic!("expected abi_type to be path got: {:?}", attr)
        }
    }

    let abi_type = if let Some(tokens) = abi_type_name {
        tokens
    } else {
        panic!("#[derive(IsAbiCompatible)] required abi_type to be defined")
    };

    // Construct the abi type path string
    let abi_type_name = abi_type
        .segments
        .iter()
        .map(|s| format!("{}", s.ident))
        .collect::<Vec<_>>()
        .join("::");

    // Get the type and name of the struct we're implementing this for
    let struct_type = &derive_input.ident;
    let struct_type_name = format!("{}", struct_type);

    // Generate code for every field to test its compatibility
    let field_types = struct_data.fields.iter().map(|f| {
        let ty = &f.ty;
        let name = f.ident.as_ref().unwrap().to_string();
        let ident = f.ident.as_ref().unwrap();
        quote! {
            self::test::AbiTypeHelper::from_value(&abi_value.#ident)
                .ir_type::<#ty>()
                .assert_compatible(#struct_type_name, #abi_type_name, #name);
        }
    });

    // Generate Phase
    (quote! {
        #[cfg(test)]
        impl self::test::TestIsAbiCompatible<#abi_type> for #struct_type {
            fn test(abi_value: &#abi_type) {
                use self::test::*;
                #(#field_types)*
            }
        }

        #[cfg(test)]
        impl self::test::IsAbiCompatible<#abi_type> for #struct_type {}

        #[cfg(test)]
        impl self::test::IsAbiCompatible<#struct_type> for #struct_type {}
    })
    .into()
}
