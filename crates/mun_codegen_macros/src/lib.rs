#![cfg(not(tarpaulin_include))]

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput};

/// This procedural macro implements the `AsValue` trait as well as several required other traits.
/// All of these traits enable creating an `inkwell::values::StructValue` from a generic struct, as
/// long as all fields of the struct also implement `AsValue`.
#[proc_macro_derive(AsValue)]
pub fn as_value_derive(input: TokenStream) -> TokenStream {
    // Parse Phase
    let derive_input = parse_macro_input!(input as DeriveInput);

    // Get the typename of the struct we're working with
    let ident = {
        let ident = derive_input.ident;
        let generics = derive_input.generics;
        quote! {
            #ident #generics
        }
    };

    match derive_input.data {
        Data::Struct(struct_data) => {
            // Generate a list of where clauses that ensure that we can cast each field to an
            // `inkwell::types::BasicTypeEnum`
            let field_types = struct_data.fields.iter().map(|f| {
                let ty = &f.ty;
                quote! {
                    Into::<inkwell::types::BasicTypeEnum<'ink>>::into(<#ty>::get_ir_type(context))
                }
            });

            // Generate a list of where clauses that ensure that we can cast each field to an
            // `inkwell::values::BasicTypeValue`
            let field_types_values = struct_data.fields.iter().enumerate().map(|(idx, f)| {
                let name = f.ident.as_ref().map(|i| quote! { #i }).unwrap_or_else(|| quote! { #idx });
                quote! {
                    {
                        let value = crate::value::AsValueInto::<'ink, inkwell::values::BasicValueEnum<'ink>>::as_value_into(&self. #name, context);
                        // eprintln!("- {:?}", value.get_type());
                        value
                    }
                }
            });

            // Generate Phase
            (quote! {
                impl<'ink> crate::value::ConcreteValueType<'ink> for #ident {
                    type Value = inkwell::values::StructValue<'ink>;
                }

                impl<'ink> crate::value::SizedValueType<'ink> for #ident {
                    fn get_ir_type(context: &crate::value::IrTypeContext<'ink, '_>) -> inkwell::types::StructType<'ink> {
                        let key = std::any::type_name::<#ident>();
                        match context.struct_types.borrow().get(&key) {
                            Some(value) => {
                                return *value;
                            }
                            None => (),
                        };

                        let struct_ty = context.context.opaque_struct_type(key);
                        context.struct_types.borrow_mut().insert(key, struct_ty);
                        struct_ty.set_body(&[
                            #(#field_types),*
                        ], false);
                        struct_ty
                    }
                }

                impl<'ink> crate::value::PointerValueType<'ink> for #ident {
                    fn get_ptr_type(context: &crate::value::IrTypeContext<'ink, '_>, address_space: Option<inkwell::AddressSpace>) -> inkwell::types::PointerType<'ink> {
                        Self::get_ir_type(context).ptr_type(address_space.unwrap_or(inkwell::AddressSpace::Generic))
                    }
                }

                impl<'ink> crate::value::AsValue<'ink, #ident> for #ident {
                    fn as_value(&self, context: &crate::value::IrValueContext<'ink, '_, '_>) -> crate::value::Value<'ink, Self> {
                        let struct_type = Self::get_ir_type(context.type_context);
                        // eprintln!("Constructing: {:?}", struct_type.print_to_string().to_string());
                        let value = struct_type.const_named_struct(&[
                            #(#field_types_values),*
                        ]);
                        // eprintln!("Done");
                        crate::value::Value::from_raw(value)
                    }
                }

                impl<'ink> crate::value::AddressableType<'ink, #ident> for #ident {}
            }).into()
        }
        Data::Union(_) => {
            unimplemented!("#[derive(AsValue)] is not defined for unions!");
        }
        Data::Enum(enum_data) => {
            // eprintln!("- {:?}", enum_data.variants);

            let variant_sizes = enum_data.variants.iter().map(|v| {
                let field_sizes = v.fields.iter().map(|f| {
                    let ty = &f.ty;
                    quote! {{
                        let ir_type = <#ty>::get_ir_type(type_context);
                        type_context.target_data.get_store_size(&ir_type)
                    }}
                });

                let variant_size = quote! {{
                    let field_sizes = [#(#field_sizes),*];
                    field_sizes.iter().sum()
                }};

                variant_size
            });

            let tag_size = quote! {
                4u64
            };

            let num_chunks = quote! {{
                let variant_sizes = [#(#variant_sizes),*];
                let max_size = variant_sizes.iter().max().cloned().unwrap_or(0);

                fn number_of_chunks(chunk_size: u64, data_size: u64) -> u64 {
                    (data_size + chunk_size - 1) / chunk_size
                }

                number_of_chunks(#tag_size, max_size)
            }};

            // Generate Phase
            (quote! {
                impl<'ink> crate::value::ConcreteValueType<'ink> for #ident {
                    type Value = inkwell::values::StructValue<'ink>;
                }

                impl<'ink> crate::value::SizedValueType<'ink> for #ident {
                    fn get_ir_type(
                        context: &crate::value::IrTypeContext<'ink, '_>
                    ) -> inkwell::types::StructType<'ink> {
                        use std::convert::TryFrom;

                        let key = std::any::type_name::<#ident>();
                        if let Some(value) = context.struct_types.borrow().get(&key) {
                            return *value;
                        };

                        let struct_ty = context.context.opaque_struct_type(key);
                        context.struct_types.borrow_mut().insert(key, struct_ty);

                        let type_context = context;
                        let num_chunks = #num_chunks;
                        let num_chunks = u32::try_from(num_chunks).expect(
                            &format!("Number of chunks is too large: {}", num_chunks)
                        );
                        let chunk_ty = context.context.i32_type();

                        struct_ty.set_body(&[
                            <[u32; 0]>::get_ir_type(context).into(),
                            <u32>::get_ir_type(context).into(),
                            chunk_ty.array_type(num_chunks).into(),
                        ], false);

                        struct_ty
                    }
                }

                impl<'ink> crate::value::PointerValueType<'ink> for #ident {
                    fn get_ptr_type(context: &crate::value::IrTypeContext<'ink, '_>, address_space: Option<inkwell::AddressSpace>) -> inkwell::types::PointerType<'ink> {
                        Self::get_ir_type(context).ptr_type(address_space.unwrap_or(inkwell::AddressSpace::Generic))
                    }
                }

                impl<'ink> crate::value::AsValue<'ink, #ident> for #ident {
                    fn as_value(&self, context: &crate::value::IrValueContext<'ink, '_, '_>) -> crate::value::Value<'ink, Self> {
                        let type_context = context.type_context;
                        let struct_type = Self::get_ir_type(type_context);

                        let chunk_ty = context.context.i32_type();
                        let num_chunks = #num_chunks;
                        let tag_size = #tag_size;

                        let chunks: Vec<_> = {
                            let chunk_ptr = self as *const Self as *const u32;
                            let chunks = unsafe {
                                std::slice::from_raw_parts(chunk_ptr, (num_chunks + 1) as usize)
                            };

                            chunks
                                .iter()
                                .map(|c| chunk_ty.const_int(u64::from(*c), false))
                                .collect()
                        };

                        let value = struct_type.const_named_struct(&[
                            chunk_ty.const_array(&[]).into(),
                            chunks[0].into(),
                            chunk_ty.const_array(&chunks[1..]).into(),
                        ]);

                        crate::value::Value::from_raw(value)
                    }
                }

                impl<'ink> crate::value::AddressableType<'ink, #ident> for #ident {}
            }).into()
        }
    }
}
