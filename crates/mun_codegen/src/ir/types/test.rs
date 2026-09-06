use std::mem;

use inkwell::types::AnyType;
use mun_abi as abi;

use super::AbiTypes;

#[test]
fn abi_struct_sizes() {
    fn assert_layout<'ink, A, T: AnyType<'ink>>(
        target_data: &inkwell::targets::TargetData,
        llvm_type: &T,
    ) {
        assert_eq!(
            mem::size_of::<A>(),
            target_data.get_abi_size(llvm_type) as usize,
            "size mismatch for {}",
            std::any::type_name::<A>(),
        );
        assert_eq!(
            mem::align_of::<A>(),
            target_data.get_abi_alignment(llvm_type) as usize,
            "alignment mismatch for {}",
            std::any::type_name::<A>(),
        );
    }

    let target = mun_target::spec::Target::host_target().expect("unable to determine host target");
    let target_data = inkwell::targets::TargetData::create(&target.data_layout);
    let context = inkwell::context::Context::create();
    let types = AbiTypes::new(&context, &target_data);

    let guid = context
        .i8_type()
        .array_type(mem::size_of::<abi::Guid>() as u32);
    assert_layout::<abi::Guid, _>(&target_data, &guid);
    assert_layout::<abi::Privacy, _>(&target_data, &context.i8_type());
    assert_layout::<abi::StructMemoryKind, _>(&target_data, &context.i8_type());
    assert_layout::<abi::TypeId<'_>, _>(&target_data, &types.type_id);
    assert_layout::<abi::PointerTypeId<'_>, _>(&target_data, &types.pointer_type_id_type());
    assert_layout::<abi::ArrayTypeId<'_>, _>(&target_data, &types.array_type_id_type());
    assert_layout::<abi::TypeDefinitionData<'_>, _>(&target_data, &types.type_definition_data);
    assert_layout::<abi::StructDefinition<'_>, _>(&target_data, &types.struct_definition);
    assert_layout::<abi::TypeDefinition<'_>, _>(&target_data, &types.type_definition);
    assert_layout::<abi::FunctionSignature<'_>, _>(&target_data, &types.function_signature);
    assert_layout::<abi::FunctionPrototype<'_>, _>(&target_data, &types.function_prototype);
    assert_layout::<abi::FunctionDefinition<'_>, _>(&target_data, &types.function_definition);
    assert_layout::<abi::ModuleInfo<'_>, _>(&target_data, &types.module_info);
    assert_layout::<abi::DispatchTable<'_>, _>(&target_data, &types.dispatch_table);
    assert_layout::<abi::TypeLut<'_>, _>(&target_data, &types.type_lut);
    assert_layout::<abi::AssemblyInfo<'_>, _>(&target_data, &types.assembly_info);
}
