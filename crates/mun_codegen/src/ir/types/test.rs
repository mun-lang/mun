use inkwell::types::AnyType;
use mun_abi as abi;
use std::cell::RefCell;
use std::mem;

use crate::ir::types as ir;
use crate::value::{IrTypeContext, SizedValueType};

#[test]
fn abi_struct_sizes() {
    fn test_type_size<'ink, A: Sized, T: SizedValueType<'ink>>(context: &IrTypeContext<'ink, '_>) {
        let ir_type = T::get_ir_type(context);
        println!("{}", ir_type.print_to_string().to_string());
        let ir_size = context.target_data.get_abi_size(&ir_type);
        assert_eq!(mem::size_of::<A>(), ir_size as usize);
    }

    // Get target data for the current host
    let target = mun_target::spec::Target::host_target().expect("unable to determine host target");
    let target_data = inkwell::targets::TargetData::create(&target.data_layout);

    // Create an LLVM context and type context to work with.
    let context = inkwell::context::Context::create();
    let type_context = IrTypeContext {
        context: &context,
        target_data: &target_data,
        struct_types: &RefCell::default(),
    };

    test_type_size::<abi::Guid, abi::Guid>(&type_context);
    test_type_size::<abi::Privacy, abi::Privacy>(&type_context);
    test_type_size::<abi::StructMemoryKind, abi::StructMemoryKind>(&type_context);
    test_type_size::<abi::TypeId<'_>, ir::TypeId<'_>>(&type_context);
    test_type_size::<abi::PointerTypeId<'_>, ir::PointerTypeId<'_>>(&type_context);
    test_type_size::<abi::ArrayTypeId<'_>, ir::ArrayTypeId<'_>>(&type_context);
    test_type_size::<abi::TypeDefinitionData<'_>, ir::TypeDefinitionData<'_>>(&type_context);
    test_type_size::<abi::StructDefinition<'_>, ir::StructDefinition<'_>>(&type_context);
    test_type_size::<abi::TypeDefinition<'_>, ir::TypeDefinition<'_>>(&type_context);
    test_type_size::<abi::FunctionSignature<'_>, ir::FunctionSignature<'_>>(&type_context);
    test_type_size::<abi::FunctionPrototype<'_>, ir::FunctionPrototype<'_>>(&type_context);
    test_type_size::<abi::ModuleInfo<'_>, ir::ModuleInfo<'_>>(&type_context);
    test_type_size::<abi::DispatchTable<'_>, ir::DispatchTable<'_>>(&type_context);
    test_type_size::<abi::TypeLut<'_>, ir::TypeLut<'_>>(&type_context);
    test_type_size::<abi::AssemblyInfo<'_>, ir::AssemblyInfo<'_>>(&type_context);
}
