use inkwell::{
    builder::Builder,
    types::{BasicTypeEnum, FunctionType},
    values::{
        BasicMetadataValueEnum, BasicValueEnum, CallSiteValue, CallableValue, FunctionValue,
        InstructionValue, PointerValue,
    },
    AddressSpace,
};
use mun_hir::Ty;

/// A function pointer paired with the signature required to call it.
#[derive(Clone, Copy)]
pub(crate) struct Callable<'ink> {
    pointer: PointerValue<'ink>,
    signature: FunctionType<'ink>,
}

impl<'ink> Callable<'ink> {
    pub(crate) fn new(pointer: PointerValue<'ink>, signature: FunctionType<'ink>) -> Self {
        debug_assert_eq!(
            pointer.get_type(),
            signature.ptr_type(AddressSpace::default())
        );
        Self { pointer, signature }
    }

    pub(crate) fn from_function(function: FunctionValue<'ink>) -> Self {
        Self::new(
            function.as_global_value().as_pointer_value(),
            function.get_type(),
        )
    }

    pub(crate) fn call(
        self,
        builder: &Builder<'ink>,
        args: &[BasicMetadataValueEnum<'ink>],
        name: &str,
    ) -> CallSiteValue<'ink> {
        debug_assert_eq!(
            self.pointer.get_type(),
            self.signature.ptr_type(AddressSpace::default())
        );
        let callable = CallableValue::try_from(self.pointer)
            .expect("callable pointer must have a function signature");
        builder.build_call(callable, args, name)
    }
}

/// An SSA value paired with the Mun type that gives the value its meaning.
#[derive(Clone)]
pub(crate) struct Operand<'ink> {
    value: BasicValueEnum<'ink>,
    ty: Ty,
}

impl<'ink> Operand<'ink> {
    pub(crate) fn new(value: BasicValueEnum<'ink>, ty: Ty) -> Self {
        Self { value, ty }
    }

    pub(crate) fn value(&self) -> BasicValueEnum<'ink> {
        self.value
    }

    pub(crate) fn into_value(self) -> BasicValueEnum<'ink> {
        self.value
    }

    pub(crate) fn ty(&self) -> &Ty {
        &self.ty
    }
}

/// The LLVM address and pointee type required to access a memory location.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) struct PlaceValue<'ink> {
    pointer: PointerValue<'ink>,
    pointee: BasicTypeEnum<'ink>,
}

impl<'ink> PlaceValue<'ink> {
    pub(crate) fn new(pointer: PointerValue<'ink>, pointee: BasicTypeEnum<'ink>) -> Self {
        Self { pointer, pointee }
    }

    pub(crate) fn pointer(self) -> PointerValue<'ink> {
        self.pointer
    }

    pub(crate) fn pointee(self) -> BasicTypeEnum<'ink> {
        self.pointee
    }
    pub(crate) fn load(self, builder: &Builder<'ink>, name: &str) -> BasicValueEnum<'ink> {
        builder.build_load(self.pointer, name)
    }

    pub(crate) fn store(
        self,
        builder: &Builder<'ink>,
        value: BasicValueEnum<'ink>,
    ) -> InstructionValue<'ink> {
        debug_assert_eq!(self.pointee, value.get_type());
        builder.build_store(self.pointer, value)
    }
}

/// A writable memory location paired with its Mun type.
#[derive(Clone)]
pub(crate) struct Place<'ink> {
    value: PlaceValue<'ink>,
    ty: Ty,
}

impl<'ink> Place<'ink> {
    pub(crate) fn new(value: PlaceValue<'ink>, ty: Ty) -> Self {
        Self { value, ty }
    }

    pub(crate) fn value(&self) -> PlaceValue<'ink> {
        self.value
    }

    pub(crate) fn ty(&self) -> &Ty {
        &self.ty
    }

    pub(crate) fn load(&self, builder: &Builder<'ink>, name: &str) -> Operand<'ink> {
        Operand::new(self.value.load(builder, name), self.ty.clone())
    }

    pub(crate) fn store(
        &self,
        builder: &Builder<'ink>,
        operand: &Operand<'ink>,
    ) -> InstructionValue<'ink> {
        debug_assert_eq!(self.ty(), operand.ty());
        self.value.store(builder, operand.value())
    }

    pub(crate) fn store_value(
        &self,
        builder: &Builder<'ink>,
        value: BasicValueEnum<'ink>,
    ) -> InstructionValue<'ink> {
        self.value.store(builder, value)
    }
}
