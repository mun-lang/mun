#![cfg(not(tarpaulin_include))]

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Ident, Index};

/// This procedural macro implements the `AsValue` trait as well as several
/// required other traits. All of these traits enable creating an
/// `inkwell::values::StructValue` from a generic struct, as long as all fields
/// of the struct also implement `AsValue`.
#[proc_macro_derive(AsValue)]
pub fn as_value_derive(input: TokenStream) -> TokenStream {
    // Parse Phase
    let derive_input = parse_macro_input!(input as DeriveInput);

    // Get the typename of the struct we're working with
    let ident = {
        let ident = &derive_input.ident;
        let generics = derive_input.generics;
        quote! {
            #ident #generics
        }
    };

    match derive_input.data {
        Data::Struct(struct_data) => {
            // Generate a list of functions that return `false` if the struct field does not
            // have an equivalent constant IR value.
            let field_has_const_values = struct_data.fields.iter().map(|f| {
                let ty = &f.ty;
                quote! {
                    if !<#ty>::has_const_value() {
                        return false;
                    }
                }
            });

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
            let field_padding_bytes = field_padding_types.clone();

            // Generate a list of where clauses that ensure that we can cast each field to
            // an `inkwell::types::BasicTypeEnum`
            let field_types = struct_data.fields.iter().map(|f| {
                let ty = &f.ty;
                quote! {
                    Into::<inkwell::types::BasicTypeEnum<'ink>>::into(<#ty>::get_ir_type(context))
                }
            });

            // Generate a list of where clauses that ensure that we can cast each field to
            // an `inkwell::values::BasicTypeValue`
            let field_types_values = struct_data.fields.iter().enumerate().map(|(idx, f)| {
                let idx = Index::from(idx);
                let name = f.ident.as_ref().map_or_else(|| quote! { #idx }, |i| quote! { #i });
                quote! {
                    {
                        let value = crate::value::AsValueInto::<'ink, inkwell::values::BasicValueEnum<'ink>>::as_value_into(&self. #name, context);
                        value
                    }
                }
            });

            // Generate a list of bytes and `inkwell::values::PointerValue`s for each field.
            //
            // Expects:
            // - type_context: &IrTypeContext
            // - fn padded_size(align: usize, data_size: usize) -> usize
            // - field_padding: Vec<usize>
            let field_bytes_and_ptrs = {
                let field_bytes_and_ptrs = struct_data.fields.iter().enumerate().map(|(idx, f)| {
                    let idx = Index::from(idx);
                    let name = f
                        .ident
                        .as_ref()
                        .map_or_else(|| quote! { #idx }, |i| quote! { #i });
                    quote! {
                        self. #name .as_bytes_and_ptrs(type_context)
                    }
                });

                quote! {{
                    let field_bytes_and_ptrs = vec![ #(#field_bytes_and_ptrs),* ];
                    field_padding
                        .into_iter()
                        .map(|p| vec![BytesOrPtr::Bytes(vec![0u8; p])])
                        // Interleave padding and field types, resulting in:
                        // [align_padding1, type1, align_padding2, type2, rear_padding]
                        .interleave(field_bytes_and_ptrs.into_iter())
                        .flatten()
                        .collect::<Vec<_>>()
                }}
            };

            // Generate Phase
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
                        Self::get_ir_type(context).ptr_type(address_space.unwrap_or(inkwell::AddressSpace::default()))
                    }
                }

                impl<'ink> crate::value::HasConstValue for #ident {
                    fn has_const_value() -> bool {
                        use crate::value::HasConstValue;
                        #(#field_has_const_values)*
                        true
                    }
                }

                impl<'ink> crate::value::AsBytesAndPtrs<'ink> for #ident {
                    fn as_bytes_and_ptrs(
                        &self,
                        context: &crate::value::IrTypeContext<'ink, '_>
                    ) -> Vec<crate::value::BytesOrPtr<'ink>> {
                        use crate::value::AsBytesAndPtrs;

                        fn padded_size(align: usize, data_size: usize) -> usize {
                            ((data_size + align - 1) / align) * align
                        }

                        // Aliasing to make sure that all procedurally generated macros can use the
                        // same variable name.
                        let type_context = context;
                        let field_padding = #field_padding_bytes;

                        #field_bytes_and_ptrs
                    }
                }

                impl<'ink> crate::value::AsValue<'ink, #ident> for #ident {
                    fn as_value(&self, context: &crate::value::IrValueContext<'ink, '_, '_>) -> crate::value::Value<'ink, Self> {
                        use crate::value::HasConstValue;

                        /// Calculates the size of data after padding has been appended to its end,
                        /// based on its alignment.
                        fn padded_size(align: usize, data_size: usize) -> usize {
                            ((data_size + align - 1) / align) * align
                        }

                        // Aliasing to make sure that all procedurally generated macros can use the
                        // same variable name.
                        let type_context = context.type_context;
                        let field_padding = #field_padding_values;

                        // If struct type can be constructed as a constant LLVM IR value
                        if <#ident>::has_const_value() {
                            // construct a named instance of that struct type
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
                            // eprintln!("Done");
                            crate::value::Value::from_raw(value)
                        } else {
                            use crate::value::{AsBytesAndPtrs, BytesOrPtr};
                            use inkwell::values::BasicValueEnum;

                            // construct an anonymous struct type consisting of bytes and pointers
                            let field_bytes_and_ptrs =  self
                                .as_bytes_and_ptrs(context.type_context)
                                .into_iter()
                                .fold(Vec::new(), |mut v, rhs| {
                                    match rhs {
                                        BytesOrPtr::Bytes(mut rhs) => {
                                            if let Some(BytesOrPtr::Bytes(lhs)) = v.last_mut() {
                                                lhs.append(&mut rhs);
                                            } else {
                                                v.push(BytesOrPtr::Bytes(rhs));
                                            }
                                        }
                                        BytesOrPtr::UntypedPtr(p) => {
                                            v.push(BytesOrPtr::UntypedPtr(p));
                                        }
                                    }
                                    v
                                });

                            let byte_ty = <u8>::get_ir_type(context.type_context);

                            let field_values: Vec<BasicValueEnum> = field_bytes_and_ptrs
                                .into_iter()
                                .map(|f| match f {
                                    BytesOrPtr::Bytes(b) => {
                                        let bytes: Vec<_> = b
                                            .into_iter()
                                            .map(|b| byte_ty.const_int(u64::from(b), false))
                                            .collect();

                                        byte_ty.const_array(&bytes).into()
                                    }
                                    BytesOrPtr::UntypedPtr(ptr) => ptr.into(),
                                })
                                .collect();

                            let value = context.context.const_struct(&field_values, true);
                            Value::from_raw(value)
                        }
                    }
                }

                impl<'ink> crate::value::AddressableType<'ink, #ident> for #ident {}
            }).into()
        }
        Data::Union(_) => {
            unimplemented!("#[derive(AsValue)] is not defined for unions!");
        }
        Data::Enum(enum_data) => {
            // Only allow these types in the `repr` attribute
            const SUPPORTED_TAG_SIZES: &[&str] =
                &["u8", "u16", "u32", "u64", "i8", "i16", "i32", "i64"];

            let repr_ty = derive_input.attrs.iter().find_map(|attr| {
                let mut repr_ty = None::<proc_macro2::TokenStream>;

                // Check whether the enum has a `repr` attribute
                if attr.path().is_ident("repr") {
                    attr.parse_nested_meta(|meta| {
                        // Use the `repr` attribute as tag type.
                        if let Some(segment) = meta.path.segments.iter().next() {
                            let ident = segment.ident.clone();
                            let tag_name = ident.to_string();
                            if !SUPPORTED_TAG_SIZES.contains(&tag_name.as_str()) {
                                return Err(
                                    meta.error(format!("unrecognised repr type: ${tag_name}"))
                                );
                            }

                            repr_ty = Some(quote! {
                                #ident
                            });

                            Ok(())
                        } else {
                            Err(meta.error("repr missing type. E.g. repr(u8)"))
                        }
                    })
                    .unwrap_or_else(|err| {
                        eprintln!("{err}");
                    });
                }

                repr_ty
            });

            let repr_ty = repr_ty.unwrap_or_else(|| {
                // Default to u32
                quote! {
                    u32
                }
            });

            if enum_data.variants.is_empty() {
                eprintln!("Enums with no variants are not supported by the `AsValue` macro.");
            }

            let enum_name = &derive_input.ident;

            // Returns a variant's fields' paddings and the variant's size.
            //
            // Expects:
            // - chunk_size: usize
            // - fn padded_size(align: usize, data_size: usize) -> usize
            let variant_type_field_paddings_and_sizes = enum_data.variants.iter().map(|v| {
                let field_sizes = v.fields.iter().map(|f| {
                    let ty = &f.ty;
                    quote! {{
                        let ir_type = <#ty>::get_ir_type(type_context);
                        type_context.target_data.get_store_size(&ir_type) as usize
                    }}
                });

                let field_alignments = v.fields.iter().map(|f| {
                    let ty = &f.ty;
                    quote! {{
                        let ir_type = <#ty>::get_ir_type(type_context);
                        type_context.target_data.get_preferred_alignment(&ir_type) as usize
                    }}
                });

                quote! {{
                    // Start with the tag's size (same as chunk_size)
                    let mut total_size = chunk_size;

                    let field_sizes = [ #(#field_sizes),* ];
                    let field_alignments = [ #(#field_alignments),* ];

                    // Calculate the padding required to align each field
                    let field_paddings: Vec<usize> = field_sizes
                        .iter()
                        .zip(field_alignments.iter())
                        .map(|(size, align)| {
                            let padded_size = padded_size(*align, total_size);
                            let padding = padded_size - total_size;
                            total_size = padded_size + size;
                            padding
                        })
                        .collect();

                    (
                        field_paddings,
                        total_size,
                    )
                }}
            });

            let variant_value_field_paddings_and_sizes =
                variant_type_field_paddings_and_sizes.clone();

            let variant_type_alignments = enum_data.variants.iter().map(|v| {
                let field_alignments = v.fields.iter().map(|f| {
                    let ty = &f.ty;
                    quote! {{
                        let ir_type = <#ty>::get_ir_type(type_context);
                        type_context.target_data.get_preferred_alignment(&ir_type) as usize
                    }}
                });

                let variant_align = quote! {{
                    let field_alignments = [#(#field_alignments),*];
                    field_alignments.iter().max().cloned().unwrap_or(1)
                }};

                variant_align
            });

            let variant_value_alignments = variant_type_alignments.clone();

            // Generate a list of bytes and `inkwell::values::PointerValue`s for each field.
            //
            // Expects:
            // - type_context: &IrTypeContext
            // - enum_size: usize
            // - variant_sizes: Vec<usize>
            let variant_bytes_and_ptrs = {
                let variant_bytes_and_ptrs_mapping = enum_data
                    .variants
                    .iter()
                    .enumerate()
                    .map(|(tag, v)| {
                        let tag = Index::from(tag);
                        let field_mappings = v.fields.iter().enumerate().map(|(idx, f)| {
                            let name = f.ident.as_ref().map_or_else(|| {
                                // If this is a tuple struct, map the index to an alias (e.g. 0: t0)
                                let concatenated = format!("t{idx}");
                                let local = Ident::new(&concatenated, Span::call_site());
                                let idx = Index::from(idx);
                                quote! { #idx: #local }
                            }, |i| quote! { #i });

                            name
                        });

                        let field_bytes_and_ptrs = v.fields.iter().enumerate().map(|(idx, f)| {
                            let name = f.ident.as_ref().map_or_else(|| {
                                // If this is a tuple struct, map the use an alias (e.g. t0 for 0)
                                let concatenated = format!("t{idx}");
                                let local = Ident::new(&concatenated, Span::call_site());
                                quote! { #local }
                            }, |i| quote! { #i });

                            quote! {
                                #name .as_bytes_and_ptrs(type_context)
                            }
                        });

                        let ident = &v.ident;
                        quote! {
                            #enum_name :: #ident { #(#field_mappings),* } => {
                                let (variant_field_paddings, variant_size) =
                                    variant_field_paddings_and_sizes.get(#tag).expect(
                                        "Number of `variant_field_paddings_and_sizes` does not match the number of variants."
                                    );

                                let variant_field_paddings = variant_field_paddings
                                    .iter()
                                    .map(|p| vec![0u8; *p].into());

                                let field_bytes_and_ptrs = vec![
                                    // Convert the tag to bytes
                                    vec![BytesOrPtr::Bytes(
                                        bytemuck::cast_ref::<#repr_ty, [u8; std::mem::size_of::<#repr_ty>()]>(&#tag)
                                            .to_vec()
                                    )],
                                    // Converts all other fields to bytes and pointers
                                    #(#field_bytes_and_ptrs),*
                                ];
                                let mut field_bytes_and_ptrs: Vec<_> = field_bytes_and_ptrs
                                    .iter()
                                    .flatten()
                                    .cloned()
                                    // Interleave field bytes and padding bytes, resulting in:
                                    // [tag, align_padding1, type1, align_padding2, type2]
                                    .interleave(variant_field_paddings)
                                    .collect();

                                // Calculate the rear padding required to fill all of the struct's
                                // memory.
                                let rear_padding = enum_size - variant_size;
                                field_bytes_and_ptrs.push(vec![0u8; rear_padding].into());

                                field_bytes_and_ptrs
                            }
                        }
                    });

                quote! {
                    match self {
                        #(#variant_bytes_and_ptrs_mapping)*
                    }
                }
            };

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
                        use inkwell::types::AnyType;

                        let key = std::any::type_name::<#ident>();
                        if let Some(value) = context.struct_types.borrow().get(&key) {
                            return *value;
                        };

                        // Aliasing to make sure that all procedurally generated macros can use the
                        // same variable name.
                        let type_context = context;

                        // Insert an opaque struct type to fix self referential types.
                        let struct_ty = type_context.context.opaque_struct_type(&key);
                        type_context.struct_types.borrow_mut().insert(key, struct_ty);

                        // The chunk size is the same as the tag's size
                        let chunk_ty = <#repr_ty>::get_ir_type(type_context);
                        let chunk_size = std::mem::size_of::<#repr_ty>();

                        let variant_alignments = [#(#variant_type_alignments),*];
                        let max_align = core::cmp::max(
                            chunk_size,
                            variant_alignments.iter().max().cloned().unwrap_or(1),
                        );

                        fn padded_size(align: usize, data_size: usize) -> usize {
                            ((data_size + align - 1) / align) * align
                        }

                        let variant_field_paddings_and_sizes = [ #(#variant_type_field_paddings_and_sizes),* ];
                        let max_size = variant_field_paddings_and_sizes
                            .iter()
                            .map(|(_, s)| *s)
                            .max()
                            .unwrap_or(0);

                        // Add padding for the end of the variant
                        let enum_size = padded_size(chunk_size, max_size);

                        // The tag is excluded from the number of chunks
                        let num_chunks = enum_size / chunk_size - 1;
                        let num_chunks = u32::try_from(num_chunks).expect(
                            "Number of chunks is too large (max: `u32::max()`)"
                        );

                        struct_ty.set_body(&[
                            <[#repr_ty; 0]>::get_ir_type(type_context).into(),
                            chunk_ty.into(),
                            chunk_ty.array_type(num_chunks).into(),
                        ], true);

                        struct_ty
                    }
                }

                impl<'ink> crate::value::PointerValueType<'ink> for #ident {
                    fn get_ptr_type(context: &crate::value::IrTypeContext<'ink, '_>, address_space: Option<inkwell::AddressSpace>) -> inkwell::types::PointerType<'ink> {
                        Self::get_ir_type(context).ptr_type(address_space.unwrap_or(inkwell::AddressSpace::default()))
                    }
                }

                impl<'ink> crate::value::HasConstValue for #ident {
                    fn has_const_value() -> bool {
                        false
                    }
                }

                impl<'ink> crate::value::AsBytesAndPtrs<'ink> for #ident {
                    fn as_bytes_and_ptrs(
                        &self,
                        context: &crate::value::IrTypeContext<'ink, '_>
                    ) -> Vec<crate::value::BytesOrPtr<'ink>> {
                        use crate::value::{AsBytesAndPtrs, BytesOrPtr};
                        use inkwell::types::AnyType;

                        // Aliasing to make sure that all procedurally generated macros can use the
                        // same variable name.
                        let type_context = context;

                        // The chunk size is the same as the tag's size
                        let chunk_ty = <#repr_ty>::get_ir_type(type_context);
                        let chunk_size = std::mem::size_of::<#repr_ty>();

                        let variant_alignments = [#(#variant_value_alignments),*];
                        let max_align = core::cmp::max(
                            chunk_size,
                            variant_alignments.iter().max().cloned().unwrap_or(1),
                        );

                        fn padded_size(align: usize, data_size: usize) -> usize {
                            ((data_size + align - 1) / align) * align
                        }

                        let variant_field_paddings_and_sizes = [ #(#variant_value_field_paddings_and_sizes),* ];

                        let max_size = variant_field_paddings_and_sizes
                            .iter()
                            .map(|(_, s)| *s)
                            .max()
                            .unwrap_or(0);

                        // Add padding for the end of the variant
                        let enum_size = padded_size(chunk_size, max_size);

                        #variant_bytes_and_ptrs
                    }
                }

                impl<'ink> crate::value::AsValue<'ink, #ident> for #ident {
                    fn as_value(&self, context: &crate::value::IrValueContext<'ink, '_, '_>) -> crate::value::Value<'ink, Self> {
                        use crate::value::{AsBytesAndPtrs, BytesOrPtr};
                        use inkwell::values::BasicValueEnum;
                        use inkwell::types::AnyType;

                        let field_bytes_and_ptrs =  self
                            .as_bytes_and_ptrs(context.type_context)
                            .into_iter()
                            .fold(Vec::new(), |mut v, rhs| {
                                match rhs {
                                    BytesOrPtr::Bytes(mut rhs) => {
                                        if let Some(BytesOrPtr::Bytes(lhs)) = v.last_mut() {
                                            lhs.append(&mut rhs);
                                        } else {
                                            v.push(BytesOrPtr::Bytes(rhs));
                                        }
                                    }
                                    BytesOrPtr::UntypedPtr(p) => {
                                        v.push(BytesOrPtr::UntypedPtr(p));
                                    }
                                }
                                v
                            });

                        let byte_ty = <u8>::get_ir_type(context.type_context);

                        let field_values: Vec<BasicValueEnum> = field_bytes_and_ptrs
                            .into_iter()
                            .map(|f| match f {
                                BytesOrPtr::Bytes(b) => {
                                    let bytes: Vec<_> = b
                                        .into_iter()
                                        .map(|b| byte_ty.const_int(u64::from(b), false))
                                        .collect();

                                    byte_ty.const_array(&bytes).into()
                                }
                                BytesOrPtr::UntypedPtr(ptr) => ptr.into(),
                            })
                            .collect();

                        let value = context.context.const_struct(&field_values, true);
                        Value::from_raw(value)
                    }
                }

                impl<'ink> crate::value::AddressableType<'ink, #ident> for #ident {}
            }).into()
        }
    }
}
