use crate::value::{ConcreteValueType, Value};

/// A trait implemented by a type to check if its values match its corresponding abi type (T). This
/// trait can be derived. e.g.:
///
/// ```ignore,rust
/// #[derive(AsValue, TestIsAbiCompatible)]
/// #[ir_name = "struct.MunTypeInfo"]
/// #[abi_type(abi::TypeInfo)]
/// pub struct TypeInfo { }
/// ```
///
/// The procedural macro calls for every field:
///
/// ```ignore,rust
/// self::test::AbiTypeHelper::from_value(&abi_value.<field_name>)
///     .ir_type::<<field_type>>()
///     .assert_compatible(<struct_type_name>, <abi_type_name>, <name>);
/// ```
pub trait TestIsAbiCompatible<T> {
    /// Runs a test to see if the implementor is compatible with the specified abi type.
    fn test(abi_value: &T);
}

/// A trait that is implemented if a type is ABI compatible with `T`.
pub trait IsAbiCompatible<T> {}

/// A trait indicating that a type is *not* compatible. This is a helper trait used to determine
/// if a type is ABI compatible at runtime. By default this trait is implemented for all types.
pub trait IsNotAbiCompatible {
    fn assert_compatible(&self, ir_type: &str, abi_type: &str, field_name: &str) {
        panic!(
            "the field '{}' on type '{}' is not compatible with the ABI version of the struct ({})",
            field_name, ir_type, abi_type
        );
    }
}
impl<T> IsNotAbiCompatible for T {}

/// Helper structs that allows extracting the type of a value.
pub struct AbiTypeHelper<T>(std::marker::PhantomData<T>);
pub struct AbiAndIrTypeHelper<T, S>(std::marker::PhantomData<T>, std::marker::PhantomData<S>);

impl<T> AbiTypeHelper<T> {
    pub fn from_value(_: &T) -> Self {
        Self(Default::default())
    }

    pub fn ir_type<S>(&self) -> AbiAndIrTypeHelper<S, T> {
        AbiAndIrTypeHelper(Default::default(), Default::default())
    }
}

impl<S, T: IsAbiCompatible<S>> AbiAndIrTypeHelper<T, S> {
    pub fn assert_compatible(&self, _ir_type: &str, _abi_type: &str, _field_name: &str) {}
}

impl IsAbiCompatible<u8> for u8 {}
impl IsAbiCompatible<u16> for u16 {}
impl IsAbiCompatible<u32> for u32 {}
impl IsAbiCompatible<abi::Guid> for abi::Guid {}
impl IsAbiCompatible<abi::TypeGroup> for abi::TypeGroup {}
impl IsAbiCompatible<abi::StructMemoryKind> for abi::StructMemoryKind {}
impl IsAbiCompatible<*const ::std::os::raw::c_char> for *const u8 {}
impl IsAbiCompatible<*const ::std::os::raw::c_void> for *const fn() {}
impl<S, T: IsAbiCompatible<S>> IsAbiCompatible<*const S> for *const T {}
impl<S, T: IsAbiCompatible<S>> IsAbiCompatible<*mut S> for *mut T {}
impl<S, T: ConcreteValueType> IsAbiCompatible<S> for Value<T> where T: IsAbiCompatible<S> {}

#[test]
#[cfg(test)]
fn test_type_info_abi_compatible() {
    let abi_type = abi::TypeInfo {
        guid: abi::Guid {
            b: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        },
        name: std::ptr::null(),
        size_in_bits: 0,
        alignment: 0,
        group: abi::TypeGroup::FundamentalTypes,
    };

    super::TypeInfo::test(&abi_type);
}

#[test]
#[cfg(test)]
fn test_function_signature_abi_compatible() {
    let abi_type = abi::FunctionSignature {
        arg_types: std::ptr::null(),
        return_type: std::ptr::null(),
        num_arg_types: 0,
    };

    super::FunctionSignature::test(&abi_type);
}

#[test]
#[cfg(test)]
fn test_function_prototype_abi_compatible() {
    let abi_type = abi::FunctionPrototype {
        name: std::ptr::null(),
        signature: abi::FunctionSignature {
            arg_types: std::ptr::null(),
            return_type: std::ptr::null(),
            num_arg_types: 0,
        },
    };

    super::FunctionPrototype::test(&abi_type);
}

#[test]
#[cfg(test)]
fn test_function_definition_abi_compatible() {
    let abi_type = abi::FunctionDefinition {
        prototype: abi::FunctionPrototype {
            name: std::ptr::null(),
            signature: abi::FunctionSignature {
                arg_types: std::ptr::null(),
                return_type: std::ptr::null(),
                num_arg_types: 0,
            },
        },
        fn_ptr: std::ptr::null(),
    };

    super::FunctionDefinition::test(&abi_type);
}

#[test]
#[cfg(test)]
fn test_struct_info_abi_compatible() {
    let abi_type = abi::StructInfo {
        field_names: std::ptr::null(),
        field_types: std::ptr::null(),
        field_offsets: std::ptr::null(),
        num_fields: 0,
        memory_kind: abi::StructMemoryKind::Value,
    };

    super::StructInfo::test(&abi_type);
}

#[test]
#[cfg(test)]
fn test_module_info_abi_compatible() {
    let abi_type = abi::ModuleInfo {
        path: std::ptr::null(),
        functions: std::ptr::null(),
        num_functions: 0,
        types: std::ptr::null(),
        num_types: 0,
    };

    super::ModuleInfo::test(&abi_type);
}

#[test]
#[cfg(test)]
fn test_dispatch_table_abi_compatible() {
    let abi_type = abi::DispatchTable {
        prototypes: std::ptr::null(),
        fn_ptrs: std::ptr::null_mut(),
        num_entries: 0,
    };

    super::DispatchTable::test(&abi_type);
}

#[test]
#[cfg(test)]
fn test_assembly_info_abi_compatible() {
    let abi_type = abi::AssemblyInfo {
        symbols: abi::ModuleInfo {
            path: std::ptr::null(),
            functions: std::ptr::null(),
            num_functions: 0,
            types: std::ptr::null(),
            num_types: 0,
        },
        dispatch_table: abi::DispatchTable {
            prototypes: std::ptr::null(),
            fn_ptrs: std::ptr::null_mut(),
            num_entries: 0,
        },
        dependencies: std::ptr::null(),
        num_dependencies: 0,
    };

    super::AssemblyInfo::test(&abi_type);
}
