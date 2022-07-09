use inkwell::types::AnyType;
use std::cell::RefCell;
use std::mem;

use crate::ir::types as ir;
use crate::value::{IrTypeContext, SizedValueType};

#[test]
fn abi_struct_sizes() {
    // Get target data for the current host
    let target = mun_target::spec::Target::host_target().expect("unable to determine host target");
    let target_data = inkwell::targets::TargetData::create(&target.data_layout);

    // Create an LLVM context and type context to work with.
    let context = inkwell::context::Context::create();
    let type_context = IrTypeContext {
        context: &context,
        target_data: &target_data,
        struct_types: &RefCell::new(Default::default()),
    };

    fn test_type_size<'ink, A: Sized, T: SizedValueType<'ink>>(context: &IrTypeContext<'ink, '_>) {
        let ir_type = T::get_ir_type(context);
        println!("{}", ir_type.print_to_string().to_string());
        let ir_size = context.target_data.get_abi_size(&ir_type);
        assert_eq!(mem::size_of::<A>(), ir_size as usize);
    }

    test_type_size::<abi::Guid, abi::Guid>(&type_context);
    test_type_size::<abi::Privacy, abi::Privacy>(&type_context);
    test_type_size::<abi::StructMemoryKind, abi::StructMemoryKind>(&type_context);
    test_type_size::<abi::TypeId, ir::TypeId>(&type_context);
    test_type_size::<abi::PointerTypeId, ir::PointerTypeId>(&type_context);
    test_type_size::<abi::ArrayTypeId, ir::ArrayTypeId>(&type_context);
    test_type_size::<abi::TypeDefinitionData, ir::TypeDefinitionData>(&type_context);
    test_type_size::<abi::StructDefinition, ir::StructDefinition>(&type_context);
    test_type_size::<abi::TypeDefinition, ir::TypeDefinition>(&type_context);
    test_type_size::<abi::FunctionSignature, ir::FunctionSignature>(&type_context);
    test_type_size::<abi::FunctionPrototype, ir::FunctionPrototype>(&type_context);
    test_type_size::<abi::ModuleInfo, ir::ModuleInfo>(&type_context);
    test_type_size::<abi::DispatchTable, ir::DispatchTable>(&type_context);
    test_type_size::<abi::TypeLut, ir::TypeLut>(&type_context);
    test_type_size::<abi::AssemblyInfo, ir::AssemblyInfo>(&type_context);
}
