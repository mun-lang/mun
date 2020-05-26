#![cfg_attr(tarpaulin, skip)]

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Lit, Meta};

#[proc_macro_derive(AsValue, attributes(ir_name))]
pub fn as_value_derive(input: TokenStream) -> TokenStream {
    // Parse Phase
    let derive_input = parse_macro_input!(input as DeriveInput);
    let struct_data = match derive_input.data {
        Data::Struct(data) => data,
        Data::Union(_) => panic!("#[derive(AsValue)] is only defined for structs, not for unions!"),
        Data::Enum(_) => panic!("#[derive(AsValue)] is only defined for structs, not for enums!"),
    };

    let mut ir_name = String::new();
    for attr in derive_input
        .attrs
        .iter()
        .map(Attribute::parse_meta)
        .filter_map(|x| x.ok())
    {
        if let Meta::NameValue(meta_name_value) = attr {
            match meta_name_value.lit {
                Lit::Str(lit_str) => {
                    ir_name = lit_str.value();
                }
                _ => {
                    panic!("ActualName must be a string");
                }
            };
        }
    }

    let ident = &derive_input.ident;

    let field_types_tuple = struct_data.fields.iter().map(|f| {
        let ty = &f.ty;
        quote! {
            #ty
        }
    });

    let field_types = struct_data.fields.iter().map(|f| {
        let ty = &f.ty;
        quote! {
            Into::<inkwell::types::BasicTypeEnum>::into(<#ty>::get_ir_type(context))
        }
    });

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
