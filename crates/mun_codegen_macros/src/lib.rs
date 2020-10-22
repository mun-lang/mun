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
            // Generate a list of struct fields' paddings.
            //
            // Expects:
            // - type_context: &IrTypeContext
            // - fn padded_size(align: usize, data_size: usize) -> usize
            let field_padding_types = {
                let field_sizes = struct_data.fields.iter().map(|f| {
                    let ty = &f.ty;
                    quote! {{
                        let ir_type = <#ty>::get_ir_type(type_context);
                        type_context.target_data.get_store_size(&ir_type) as usize
                    }}
                });

                let field_alignments = struct_data.fields.iter().map(|f| {
                    let ty = &f.ty;
                    quote! {{
                        let ir_type = <#ty>::get_ir_type(type_context);
                        type_context.target_data.get_preferred_alignment(&ir_type) as usize
                    }}
                });

                quote! {{
                    let mut total_size = 0;

                    let field_sizes = vec![ #(#field_sizes),* ];
                    let field_alignments = vec![ #(#field_alignments),* ];

                    let mut field_paddings: Vec<usize> = field_sizes
                        .iter()
                        .zip(field_alignments.iter())
                        .map(|(size, align)| {
                            let padded_size = padded_size(*align, total_size);
                            let padding = padded_size - total_size;
                            total_size = padded_size + size;
                            padding
                        })
                        .collect();

                    // Add padding for the end of the struct
                    let max_align = field_alignments.iter().max().cloned().unwrap_or(1);
                    let padded_size = padded_size(max_align, total_size);
                    field_paddings.push(padded_size - total_size);

                    field_paddings
                }}
            };

            let field_padding_values = field_padding_types.clone();

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
                        value
                    }
                }
            });

            // Generate functions
            (quote! {
                impl<'ink> crate::value::ConcreteValueType<'ink> for #ident {
                    type Value = inkwell::values::StructValue<'ink>;
                }

                impl<'ink> crate::value::SizedValueType<'ink> for #ident {
                    fn get_ir_type(context: &crate::value::IrTypeContext<'ink, '_>) -> inkwell::types::StructType<'ink> {
                        // Check whether the IR struct type exists
                        let key = std::any::type_name::<#ident>();
                        match context.struct_types.borrow().get(&key) {
                            Some(value) => {
                                return *value;
                            }
                            None => (),
                        };

                        // Construct a new IR struct type
                        let struct_ty = context.context.opaque_struct_type(key);
                        context.struct_types.borrow_mut().insert(key, struct_ty);

                        /// Calculates the size of data after padding has been appended to its end,
                        /// based on its alignment.
                        fn padded_size(align: usize, data_size: usize) -> usize {
                            ((data_size + align - 1) / align) * align
                        }

                        // Aliasing to make sure that all procedurally generated macros can use the
                        // same variable name.
                        let type_context = context;

                        let field_types = vec![ #(#field_types),* ];
                        let field_padding = #field_padding_types;
                        let struct_fields: Vec<_> = field_padding
                            .into_iter()
                            // Choose a field's padding type based on the size of its alignment
                            // padding
                            .map(|p| {
                                let (ty, num_chunks) = if p % 8 == 0 {
                                    (context.context.i64_type(), p / 8)
                                } else if p % 4 == 0 {
                                    (context.context.i32_type(), p / 4)
                                } else if p % 2 == 0 {
                                    (context.context.i16_type(), p / 2)
                                } else {
                                    (context.context.i8_type(), p)
                                };

                                ty.array_type(num_chunks as u32).into()
                            })
                            // Interleave padding and field types, resulting in:
                            // [align_padding1, type1, align_padding2, type2, rear_padding]
                            .interleave(field_types.into_iter())
                            .collect();

                        struct_ty.set_body(&struct_fields, true);
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
                        /// Calculates the size of data after padding has been appended to its end,
                        /// based on its alignment.
                        fn padded_size(align: usize, data_size: usize) -> usize {
                            ((data_size + align - 1) / align) * align
                        }

                        // Aliasing to make sure that all procedurally generated macros can use the
                        // same variable name.
                        let type_context = context.type_context;
                        let field_padding = #field_padding_values;

                        let struct_type = Self::get_ir_type(context.type_context);

                        let field_values = vec![ #(#field_types_values),* ];
                        let struct_fields: Vec<_> = field_padding
                            .into_iter()
                            // Choose a field's padding type based on the size of its alignment
                            // padding
                            .map(|p| {
                                let (ty, num_chunks) = if p % 8 == 0 {
                                    (context.context.i64_type(), p / 8)
                                } else if p % 4 == 0 {
                                    (context.context.i32_type(), p / 4)
                                } else if p % 2 == 0 {
                                    (context.context.i16_type(), p / 2)
                                } else {
                                    (context.context.i8_type(), p)
                                };

                                let chunks: Vec<_> = (0..num_chunks)
                                    .map(|_| ty.const_int(0, false))
                                    .collect();

                                ty.const_array(&chunks).into()
                            })
                            // Interleave padding and field types, resulting in:
                            // [align_padding1, type1, align_padding2, type2, rear_padding]
                            .interleave(field_values.into_iter())
                            .collect();

                        let value = struct_type.const_named_struct(&struct_fields);
                        crate::value::Value::from_raw(value)
                    }
                }

                impl<'ink> crate::value::AddressableType<'ink, #ident> for #ident {}
            }).into()
        }
        Data::Union(_) => {
            unimplemented!("#[derive(AsValue)] is not defined for unions!");
        }
        Data::Enum(_) => {
            unimplemented!("#[derive(AsValue)] is not defined for enums!");
        }
    }
}
