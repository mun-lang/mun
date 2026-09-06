#[cfg(test)]
mod test;

use std::{ffi::CStr, mem};

use inkwell::{
    context::Context,
    module::{Linkage, Module},
    targets::{ByteOrdering, TargetData},
    types::{FunctionType, IntType, PointerType, StructType},
    values::{
        ArrayValue, BasicValueEnum, GlobalValue, IntValue, PointerValue, StructValue,
        UnnamedAddress,
    },
    AddressSpace,
};
use mun_abi as abi;

/// The LLVM types that make up the binary interface between generated code and
/// the runtime.
///
/// This registry owns the ABI schema. Callers provide semantic data to its
/// constructors instead of manufacturing raw LLVM structs, which keeps layout
/// decisions in one place.
pub struct AbiTypes<'ink> {
    context: &'ink Context,
    byte_ordering: ByteOrdering,
    word_type: IntType<'ink>,
    word_bytes: usize,
    type_id_payload_words: u32,
    type_definition_data_payload_words: u32,
    type_id: StructType<'ink>,
    pointer_type_id: StructType<'ink>,
    array_type_id: StructType<'ink>,
    type_definition_data: StructType<'ink>,
    type_definition: StructType<'ink>,
    function_signature: StructType<'ink>,
    function_prototype: StructType<'ink>,
    function_definition: StructType<'ink>,
    struct_definition: StructType<'ink>,
    module_info: StructType<'ink>,
    dispatch_table: StructType<'ink>,
    type_lut: StructType<'ink>,
    assembly_info: StructType<'ink>,
}
/// Context used while materializing runtime ABI constants in one LLVM module.
///
/// It keeps schema, module ownership, and target layout together so ABI
/// construction does not depend on unrelated code-generation state.
pub struct AbiBuilder<'a, 'ink> {
    pub types: &'a AbiTypes<'ink>,
    pub context: &'ink Context,
    pub module: &'a Module<'ink>,
    pub target_data: &'a TargetData,
}

/// A concrete `MunTypeId` value.
///
/// The newtype prevents other ABI structs from being used where a type
/// identifier is required.
#[derive(Clone, Copy, Debug)]
pub struct TypeIdValue<'ink>(StructValue<'ink>);

/// A global containing one concrete `MunTypeId`.
#[derive(Clone, Copy, Debug)]
pub struct TypeIdGlobal<'ink>(GlobalValue<'ink>);

/// The data for the struct variant of `MunTypeDefinitionData`.
pub struct StructDefinition<'ink> {
    pub guid: abi::Guid,
    pub field_names: PointerValue<'ink>,
    pub field_types: PointerValue<'ink>,
    pub field_offsets: PointerValue<'ink>,
    pub num_fields: u16,
    pub memory_kind: abi::StructMemoryKind,
}

/// A runtime function signature encoded for reflection.
pub struct FunctionSignature<'ink> {
    pub arg_types: PointerValue<'ink>,
    pub return_type: TypeIdValue<'ink>,
    pub num_arg_types: u16,
}

/// A runtime function prototype encoded for reflection.
pub struct FunctionPrototype<'ink> {
    pub name: PointerValue<'ink>,
    pub signature: FunctionSignature<'ink>,
}

/// A runtime function definition encoded for reflection.
pub struct FunctionDefinition<'ink> {
    pub prototype: FunctionPrototype<'ink>,
    pub fn_ptr: PointerValue<'ink>,
}

/// A reflected Mun type definition.
pub struct TypeDefinition<'ink> {
    pub name: PointerValue<'ink>,
    pub size_in_bits: u32,
    pub alignment: u8,
    pub data: StructDefinition<'ink>,
}

/// Reflection information owned by one generated module.
pub struct ModuleInfo<'ink> {
    pub path: PointerValue<'ink>,
    pub functions: PointerValue<'ink>,
    pub types: PointerValue<'ink>,
    pub num_functions: u32,
    pub num_types: u32,
}

/// Reflection information for the runtime dispatch table.
pub struct DispatchTable<'ink> {
    pub prototypes: PointerValue<'ink>,
    pub fn_ptrs: PointerValue<'ink>,
    pub num_entries: u32,
}

/// Reflection information used to associate type identifiers with runtime type
/// handles.
pub struct TypeLut<'ink> {
    pub type_ids: PointerValue<'ink>,
    pub type_ptrs: PointerValue<'ink>,
    pub type_names: PointerValue<'ink>,
    pub num_entries: u32,
}

impl<'ink> AbiTypes<'ink> {
    /// Builds the complete runtime ABI schema for `context` and `target`.
    pub fn new(context: &'ink Context, target: &TargetData) -> Self {
        let address_space = AddressSpace::default();
        let word_type = context.ptr_sized_int_type(target, None);
        let word_bytes = u64::from(target.get_pointer_byte_size(None));
        let bool_type = context.bool_type();
        let byte_type = context.i8_type();
        let u16_type = context.i16_type();
        let u32_type = context.i32_type();
        let byte_ptr = byte_type.ptr_type(address_space);

        let type_id = context.opaque_struct_type("mun.abi.TypeId");
        let type_id_ptr = type_id.ptr_type(address_space);
        let pointer_type_id = context.opaque_struct_type("mun.abi.PointerTypeId");
        pointer_type_id.set_body(&[type_id_ptr.into(), bool_type.into()], false);
        let array_type_id = context.opaque_struct_type("mun.abi.ArrayTypeId");
        array_type_id.set_body(&[type_id_ptr.into()], false);

        let guid_type = byte_type.array_type(16);
        let type_id_payload_size = target
            .get_store_size(&guid_type)
            .max(target.get_store_size(&pointer_type_id))
            .max(target.get_store_size(&array_type_id));
        let type_id_payload_words = type_id_payload_size.div_ceil(word_bytes) as u32;
        type_id.set_body(
            &[
                byte_type.into(),
                byte_type.array_type((word_bytes - 1) as u32).into(),
                word_type.array_type(type_id_payload_words).into(),
            ],
            false,
        );

        let struct_definition = context.opaque_struct_type("mun.abi.StructDefinition");
        struct_definition.set_body(
            &[
                guid_type.into(),
                byte_ptr.ptr_type(address_space).into(),
                type_id_ptr.into(),
                u16_type.ptr_type(address_space).into(),
                u16_type.into(),
                byte_type.into(),
            ],
            false,
        );

        let type_definition_data = context.opaque_struct_type("mun.abi.TypeDefinitionData");
        let type_definition_data_payload_words = target
            .get_store_size(&struct_definition)
            .div_ceil(word_bytes) as u32;
        type_definition_data.set_body(
            &[
                byte_type.into(),
                byte_type.array_type((word_bytes - 1) as u32).into(),
                word_type
                    .array_type(type_definition_data_payload_words)
                    .into(),
            ],
            false,
        );

        let type_definition = context.opaque_struct_type("mun.abi.TypeDefinition");
        type_definition.set_body(
            &[
                byte_ptr.into(),
                u32_type.into(),
                byte_type.into(),
                type_definition_data.into(),
            ],
            false,
        );

        let function_signature = context.opaque_struct_type("mun.abi.FunctionSignature");
        function_signature.set_body(
            &[type_id_ptr.into(), type_id.into(), u16_type.into()],
            false,
        );

        let function_prototype = context.opaque_struct_type("mun.abi.FunctionPrototype");
        function_prototype.set_body(&[byte_ptr.into(), function_signature.into()], false);

        let function_definition = context.opaque_struct_type("mun.abi.FunctionDefinition");
        function_definition.set_body(&[function_prototype.into(), byte_ptr.into()], false);

        let module_info = context.opaque_struct_type("mun.abi.ModuleInfo");
        module_info.set_body(
            &[
                byte_ptr.into(),
                function_definition.ptr_type(address_space).into(),
                type_definition.ptr_type(address_space).into(),
                u32_type.into(),
                u32_type.into(),
            ],
            false,
        );

        let dispatch_table = context.opaque_struct_type("mun.abi.DispatchTable");
        dispatch_table.set_body(
            &[
                function_prototype.ptr_type(address_space).into(),
                byte_ptr.ptr_type(address_space).into(),
                u32_type.into(),
            ],
            false,
        );

        let type_lut = context.opaque_struct_type("mun.abi.TypeLut");
        type_lut.set_body(
            &[
                type_id_ptr.into(),
                byte_ptr.ptr_type(address_space).into(),
                byte_ptr.ptr_type(address_space).into(),
                u32_type.into(),
            ],
            false,
        );

        let assembly_info = context.opaque_struct_type("mun.abi.AssemblyInfo");
        assembly_info.set_body(
            &[
                module_info.into(),
                dispatch_table.into(),
                type_lut.into(),
                byte_ptr.ptr_type(address_space).into(),
                u32_type.into(),
            ],
            false,
        );

        Self {
            context,
            byte_ordering: target.get_byte_ordering(),
            word_type,
            word_bytes: word_bytes as usize,
            type_id_payload_words,
            type_definition_data_payload_words,
            type_id,
            pointer_type_id,
            array_type_id,
            type_definition_data,
            type_definition,
            function_signature,
            function_prototype,
            function_definition,
            struct_definition,
            module_info,
            dispatch_table,
            type_lut,
            assembly_info,
        }
    }

    pub fn type_id_type(&self) -> StructType<'ink> {
        self.type_id
    }

    pub fn assembly_info_type(&self) -> StructType<'ink> {
        self.assembly_info
    }

    pub fn function_definition_type(&self) -> StructType<'ink> {
        self.function_definition
    }

    pub fn type_definition_type(&self) -> StructType<'ink> {
        self.type_definition
    }

    pub fn function_prototype_type(&self) -> StructType<'ink> {
        self.function_prototype
    }
    /// Returns the concrete payload layout used by pointer type IDs.
    pub fn pointer_type_id_type(&self) -> StructType<'ink> {
        self.pointer_type_id
    }

    /// Returns the concrete payload layout used by array type IDs.
    pub fn array_type_id_type(&self) -> StructType<'ink> {
        self.array_type_id
    }

    pub fn concrete_type_id(&self, guid: abi::Guid) -> TypeIdValue<'ink> {
        TypeIdValue(self.enum_value(
            self.type_id,
            0,
            vec![PayloadField::bytes(guid.0.to_vec(), 1)],
            self.type_id_payload_words,
        ))
    }

    pub fn pointer_type_id(&self, pointee: TypeIdGlobal<'ink>, mutable: bool) -> TypeIdValue<'ink> {
        TypeIdValue(self.enum_value(
            self.type_id,
            1,
            vec![
                PayloadField::pointer(pointee.as_pointer_value(), self.word_bytes),
                PayloadField::bytes(vec![u8::from(mutable)], 1),
            ],
            self.type_id_payload_words,
        ))
    }

    pub fn array_type_id(&self, element: TypeIdGlobal<'ink>) -> TypeIdValue<'ink> {
        TypeIdValue(self.enum_value(
            self.type_id,
            2,
            vec![PayloadField::pointer(
                element.as_pointer_value(),
                self.word_bytes,
            )],
            self.type_id_payload_words,
        ))
    }

    pub fn struct_definition_value(&self, value: &StructDefinition<'ink>) -> StructValue<'ink> {
        self.struct_definition.const_named_struct(&[
            self.guid(value.guid).into(),
            value.field_names.into(),
            value.field_types.into(),
            value.field_offsets.into(),
            self.u16(value.num_fields).into(),
            self.u8(value.memory_kind as u8).into(),
        ])
    }

    pub fn type_definition_value(&self, value: &TypeDefinition<'ink>) -> StructValue<'ink> {
        let data = self.enum_value(
            self.type_definition_data,
            0,
            self.struct_definition_payload(&value.data),
            self.type_definition_data_payload_words,
        );
        self.type_definition.const_named_struct(&[
            value.name.into(),
            self.u32(value.size_in_bits).into(),
            self.u8(value.alignment).into(),
            data.into(),
        ])
    }

    pub fn function_signature_value(&self, value: &FunctionSignature<'ink>) -> StructValue<'ink> {
        self.function_signature.const_named_struct(&[
            value.arg_types.into(),
            value.return_type.into_struct_value().into(),
            self.u16(value.num_arg_types).into(),
        ])
    }

    pub fn function_prototype_value(&self, value: &FunctionPrototype<'ink>) -> StructValue<'ink> {
        self.function_prototype.const_named_struct(&[
            value.name.into(),
            self.function_signature_value(&value.signature).into(),
        ])
    }

    pub fn function_definition_value(&self, value: &FunctionDefinition<'ink>) -> StructValue<'ink> {
        self.function_definition.const_named_struct(&[
            self.function_prototype_value(&value.prototype).into(),
            value.fn_ptr.into(),
        ])
    }

    pub fn module_info_value(&self, value: &ModuleInfo<'ink>) -> StructValue<'ink> {
        self.module_info.const_named_struct(&[
            value.path.into(),
            value.functions.into(),
            value.types.into(),
            self.u32(value.num_functions).into(),
            self.u32(value.num_types).into(),
        ])
    }

    pub fn dispatch_table_value(&self, value: &DispatchTable<'ink>) -> StructValue<'ink> {
        self.dispatch_table.const_named_struct(&[
            value.prototypes.into(),
            value.fn_ptrs.into(),
            self.u32(value.num_entries).into(),
        ])
    }

    pub fn type_lut_value(&self, value: &TypeLut<'ink>) -> StructValue<'ink> {
        self.type_lut.const_named_struct(&[
            value.type_ids.into(),
            value.type_ptrs.into(),
            value.type_names.into(),
            self.u32(value.num_entries).into(),
        ])
    }

    pub fn assembly_info_value(
        &self,
        module: &ModuleInfo<'ink>,
        dispatch: &DispatchTable<'ink>,
        type_lut: &TypeLut<'ink>,
        dependencies: PointerValue<'ink>,
        num_dependencies: u32,
    ) -> StructValue<'ink> {
        self.assembly_info.const_named_struct(&[
            self.module_info_value(module).into(),
            self.dispatch_table_value(dispatch).into(),
            self.type_lut_value(type_lut).into(),
            dependencies.into(),
            self.u32(num_dependencies).into(),
        ])
    }

    pub fn get_info_function_type(&self, windows: bool) -> FunctionType<'ink> {
        if windows {
            self.context.void_type().fn_type(
                &[self.assembly_info.ptr_type(AddressSpace::default()).into()],
                false,
            )
        } else {
            self.assembly_info.fn_type(&[], false)
        }
    }

    pub fn intern_c_str(
        &self,
        module: &Module<'ink>,
        name: &str,
        value: &CStr,
    ) -> PointerValue<'ink> {
        let bytes = value.to_bytes_with_nul();
        let values: Vec<_> = bytes.iter().map(|&byte| self.u8(byte)).collect();
        self.private_int_array(module, name, self.context.i8_type(), &values)
    }

    pub fn private_type_id_global(
        &self,
        module: &Module<'ink>,
        name: &str,
        value: TypeIdValue<'ink>,
    ) -> TypeIdGlobal<'ink> {
        TypeIdGlobal(self.private_global(module, name, value.into_struct_value().into()))
    }

    pub fn private_type_id_array(
        &self,
        module: &Module<'ink>,
        name: &str,
        values: &[TypeIdValue<'ink>],
        null_if_empty: bool,
    ) -> PointerValue<'ink> {
        let values: Vec<_> = values
            .iter()
            .map(|value| value.into_struct_value())
            .collect();
        self.private_struct_array(module, name, self.type_id, &values, null_if_empty)
    }

    pub fn private_type_id_pointer_array(
        &self,
        module: &Module<'ink>,
        name: &str,
        values: &[TypeIdGlobal<'ink>],
        null_if_empty: bool,
    ) -> PointerValue<'ink> {
        let values: Vec<_> = values
            .iter()
            .map(|value| value.as_pointer_value())
            .collect();
        self.private_pointer_array(
            module,
            name,
            self.type_id.ptr_type(AddressSpace::default()),
            &values,
            null_if_empty,
        )
    }

    pub fn private_function_prototype_array(
        &self,
        module: &Module<'ink>,
        name: &str,
        values: &[FunctionPrototype<'ink>],
    ) -> PointerValue<'ink> {
        let values: Vec<_> = values
            .iter()
            .map(|value| self.function_prototype_value(value))
            .collect();
        self.private_struct_array(module, name, self.function_prototype, &values, false)
    }

    pub fn private_function_definition_array(
        &self,
        module: &Module<'ink>,
        name: &str,
        values: &[FunctionDefinition<'ink>],
    ) -> GlobalValue<'ink> {
        let values: Vec<_> = values
            .iter()
            .map(|value| self.function_definition_value(value))
            .collect();
        let initializer = self.function_definition.const_array(&values);
        self.private_global(module, name, initializer.into())
    }

    pub fn private_type_definition_array(
        &self,
        module: &Module<'ink>,
        name: &str,
        values: &[TypeDefinition<'ink>],
        null_if_empty: bool,
    ) -> PointerValue<'ink> {
        let values: Vec<_> = values
            .iter()
            .map(|value| self.type_definition_value(value))
            .collect();
        self.private_struct_array(module, name, self.type_definition, &values, null_if_empty)
    }

    pub fn private_pointer_array(
        &self,
        module: &Module<'ink>,
        name: &str,
        pointer_type: PointerType<'ink>,
        values: &[PointerValue<'ink>],
        null_if_empty: bool,
    ) -> PointerValue<'ink> {
        if null_if_empty && values.is_empty() {
            return pointer_type.const_null();
        }
        let initializer = pointer_type.const_array(values);
        let global = self.private_global(module, name, initializer.into());
        global.as_pointer_value().const_cast(pointer_type)
    }

    pub fn private_u16_array(
        &self,
        module: &Module<'ink>,
        name: &str,
        values: &[u16],
        null_if_empty: bool,
    ) -> PointerValue<'ink> {
        let ty = self.context.i16_type();
        let values: Vec<_> = values.iter().map(|&value| self.u16(value)).collect();
        if null_if_empty && values.is_empty() {
            return ty.ptr_type(AddressSpace::default()).const_null();
        }
        self.private_int_array(module, name, ty, &values)
    }

    pub fn private_global(
        &self,
        module: &Module<'ink>,
        name: &str,
        initializer: BasicValueEnum<'ink>,
    ) -> GlobalValue<'ink> {
        let global = module.add_global(initializer.get_type(), None, name);
        global.set_linkage(Linkage::Private);
        global.set_constant(true);
        global.set_initializer(&initializer);
        global.set_unnamed_address(UnnamedAddress::Global);
        global
    }

    fn private_int_array(
        &self,
        module: &Module<'ink>,
        name: &str,
        ty: IntType<'ink>,
        values: &[IntValue<'ink>],
    ) -> PointerValue<'ink> {
        let initializer = ty.const_array(values);
        let global = self.private_global(module, name, initializer.into());
        global
            .as_pointer_value()
            .const_cast(ty.ptr_type(AddressSpace::default()))
    }

    fn private_struct_array(
        &self,
        module: &Module<'ink>,
        name: &str,
        ty: StructType<'ink>,
        values: &[StructValue<'ink>],
        null_if_empty: bool,
    ) -> PointerValue<'ink> {
        let pointer_type = ty.ptr_type(AddressSpace::default());
        if null_if_empty && values.is_empty() {
            return pointer_type.const_null();
        }
        let initializer = ty.const_array(values);
        let global = self.private_global(module, name, initializer.into());
        global.as_pointer_value().const_cast(pointer_type)
    }

    fn enum_value(
        &self,
        ty: StructType<'ink>,
        tag: u8,
        fields: Vec<PayloadField<'ink>>,
        payload_words: u32,
    ) -> StructValue<'ink> {
        let (prefix, words) = self.payload(fields, payload_words as usize);
        ty.const_named_struct(&[
            self.u8(tag).into(),
            self.context.i8_type().const_array(&prefix).into(),
            self.word_type.const_array(&words).into(),
        ])
    }

    fn struct_definition_payload(&self, value: &StructDefinition<'ink>) -> Vec<PayloadField<'ink>> {
        vec![
            PayloadField::bytes(Vec::new(), self.word_bytes),
            PayloadField::bytes(value.guid.0.to_vec(), 1),
            PayloadField::pointer(value.field_names, self.word_bytes),
            PayloadField::pointer(value.field_types, self.word_bytes),
            PayloadField::pointer(value.field_offsets, self.word_bytes),
            PayloadField::bytes(
                self.target_bytes_u16(value.num_fields),
                mem::align_of::<u16>(),
            ),
            PayloadField::bytes(vec![value.memory_kind as u8], 1),
        ]
    }

    fn payload(
        &self,
        fields: Vec<PayloadField<'ink>>,
        num_words: usize,
    ) -> (Vec<IntValue<'ink>>, Vec<IntValue<'ink>>) {
        let word_bytes = self.word_bytes;
        let prefix_bytes = word_bytes - 1;
        let mut bytes = vec![0; prefix_bytes + num_words * word_bytes];
        let mut pointers = vec![None; num_words];
        let mut offset: usize = 1;

        for field in fields {
            offset = offset.div_ceil(field.align) * field.align;
            match field.value {
                PayloadValue::Bytes(field_bytes) => {
                    let start = offset - 1;
                    let end = start + field_bytes.len();
                    assert!(end <= bytes.len(), "ABI enum payload exceeds its storage");
                    bytes[start..end].copy_from_slice(&field_bytes);
                    offset += field_bytes.len();
                }
                PayloadValue::Pointer(pointer) => {
                    assert_eq!(offset % word_bytes, 0, "ABI pointer is not word aligned");
                    let word = offset / word_bytes - 1;
                    assert!(
                        word < pointers.len(),
                        "ABI enum pointer exceeds its storage"
                    );
                    pointers[word] = Some(pointer);
                    offset += word_bytes;
                }
            }
        }

        let prefix = bytes[..prefix_bytes]
            .iter()
            .map(|&byte| self.u8(byte))
            .collect();
        let words = bytes[prefix_bytes..]
            .chunks_exact(word_bytes)
            .enumerate()
            .map(|(index, chunk)| {
                if let Some(pointer) = pointers[index] {
                    pointer.const_to_int(self.word_type)
                } else {
                    self.word_type.const_int(self.word_from_bytes(chunk), false)
                }
            })
            .collect();
        (prefix, words)
    }

    fn word_from_bytes(&self, bytes: &[u8]) -> u64 {
        match self.byte_ordering {
            ByteOrdering::LittleEndian => {
                bytes.iter().enumerate().fold(0, |value, (index, byte)| {
                    value | (u64::from(*byte) << (index * 8))
                })
            }
            ByteOrdering::BigEndian => bytes
                .iter()
                .fold(0, |value, byte| (value << 8) | u64::from(*byte)),
        }
    }

    fn target_bytes_u16(&self, value: u16) -> Vec<u8> {
        match self.byte_ordering {
            ByteOrdering::LittleEndian => value.to_le_bytes().to_vec(),
            ByteOrdering::BigEndian => value.to_be_bytes().to_vec(),
        }
    }

    fn guid(&self, value: abi::Guid) -> ArrayValue<'ink> {
        let values: Vec<_> = value.0.iter().map(|&byte| self.u8(byte)).collect();
        self.context.i8_type().const_array(&values)
    }

    fn u8(&self, value: u8) -> IntValue<'ink> {
        self.context.i8_type().const_int(u64::from(value), false)
    }

    fn u16(&self, value: u16) -> IntValue<'ink> {
        self.context.i16_type().const_int(u64::from(value), false)
    }

    fn u32(&self, value: u32) -> IntValue<'ink> {
        self.context.i32_type().const_int(u64::from(value), false)
    }
}

impl<'ink> TypeIdValue<'ink> {
    pub fn into_struct_value(self) -> StructValue<'ink> {
        self.0
    }
}

impl<'ink> TypeIdGlobal<'ink> {
    pub fn as_pointer_value(self) -> PointerValue<'ink> {
        self.0.as_pointer_value()
    }
}

struct PayloadField<'ink> {
    align: usize,
    value: PayloadValue<'ink>,
}

impl<'ink> PayloadField<'ink> {
    fn bytes(bytes: Vec<u8>, align: usize) -> Self {
        Self {
            align,
            value: PayloadValue::Bytes(bytes),
        }
    }

    fn pointer(value: PointerValue<'ink>, word_bytes: usize) -> Self {
        Self {
            align: word_bytes,
            value: PayloadValue::Pointer(value),
        }
    }
}

enum PayloadValue<'ink> {
    Bytes(Vec<u8>),
    Pointer(PointerValue<'ink>),
}
