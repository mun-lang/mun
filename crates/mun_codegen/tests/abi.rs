use abi::{StructMemoryKind, TypeInfoData, ABI_VERSION};
use libloader::MunLibrary;
use mun_test::CompileTestDriver;
use runtime::ReturnTypeReflection;
use std::mem;

// TODO: add integration test for ModuleInfo's path
#[test]
fn test_abi_compatibility() {
    let fn_name = "foo";
    let fn_name2 = "bar";
    let struct_name = "Foo";
    let struct_name2 = "Bar";
    let driver = CompileTestDriver::from_file(&format!(
        r#"
    pub fn {fn_name}(_: f64) -> i32 {{ 0 }}
    pub fn {fn_name2}() {{ }}

    pub struct {struct_name}(f64, f64);
    pub struct(value) {struct_name2} {{ a: i32, b: i32 }};
    "#,
        fn_name = fn_name,
        fn_name2 = fn_name2,
        struct_name = struct_name,
        struct_name2 = struct_name2,
    ));

    // Assert that all library functions are exposed
    // Safety: We compiled the code ourselves, therefor loading the library is safe
    let lib = unsafe { MunLibrary::new(driver.lib_path()) }
        .expect("Failed to load generated Mun library.");

    assert_eq!(ABI_VERSION, unsafe { lib.get_abi_version() });

    let lib_info = unsafe { lib.get_info() };

    // Dependency compatibility
    assert_eq!(lib_info.num_dependencies, 0);
    // TODO: verify dependencies ABI

    assert_eq!(lib_info.dispatch_table.num_entries, 0);
    // TODO: verify dispatch table ABI

    let module_info = &lib_info.symbols;
    assert_eq!(module_info.path(), "");
    assert_eq!(module_info.num_functions, 2);

    let fn_def = get_function_info(module_info, fn_name);
    test_function_args(fn_def, &[(f64::type_name(), f64::type_id())]);
    test_function_return_type_some::<i32>(fn_def);

    let fn_def2 = get_function_info(module_info, fn_name2);
    test_function_args(fn_def2, &[]);
    test_function_return_type_none(fn_def2);

    struct Foo(f64, f64);
    test_struct_info::<Foo, f64>(
        &lib_info.symbols,
        struct_name,
        &["0", "1"],
        StructMemoryKind::Gc,
    );

    struct Bar {
        _a: i32,
        _b: i32,
    }
    test_struct_info::<Bar, i32>(
        &lib_info.symbols,
        struct_name2,
        &["a", "b"],
        StructMemoryKind::Value,
    );

    fn get_function_info<'m>(
        module_info: &'m abi::ModuleInfo,
        fn_name: &str,
    ) -> &'m abi::FunctionDefinition {
        module_info
            .functions()
            .iter()
            .find(|f| f.prototype.name() == fn_name)
            .unwrap_or_else(|| panic!("Failed to retrieve function definition '{}'", fn_name))
    }

    fn test_function_args(fn_def: &abi::FunctionDefinition, args: &[(&str, abi::TypeId)]) {
        assert_eq!(
            usize::from(fn_def.prototype.signature.num_arg_types),
            args.len()
        );

        for (idx, (_, arg_type_id)) in args.iter().enumerate() {
            let fn_arg_type = fn_def
                .prototype
                .signature
                .arg_types()
                .get(idx)
                .unwrap_or_else(|| {
                    panic!(
                        "Function '{}' should have an argument.",
                        fn_def.prototype.name()
                    )
                });

            assert_eq!(fn_arg_type, arg_type_id);
        }
    }

    #[allow(dead_code)]
    fn test_function_return_type_none(fn_def: &abi::FunctionDefinition) {
        assert!(
            fn_def.prototype.signature.return_type().is_none(),
            "Function '{}' should not have a return type.",
            fn_def.prototype.name(),
        );
    }

    fn test_function_return_type_some<R: ReturnTypeReflection>(fn_def: &abi::FunctionDefinition) {
        let fn_return_type = fn_def.prototype.signature.return_type().unwrap_or_else(|| {
            panic!(
                "Function '{}' should have a return type.",
                fn_def.prototype.name()
            )
        });
        assert_eq!(fn_return_type, R::type_id());
    }

    fn test_struct_info<T: Sized, F: Sized + ReturnTypeReflection>(
        module_info: &abi::ModuleInfo,
        struct_name: &str,
        field_names: &[&str],
        memory_kind: StructMemoryKind,
    ) {
        let type_info = module_info
            .types()
            .iter()
            .find(|ty| ty.name() == struct_name)
            .unwrap_or_else(|| panic!("Failed to retrieve struct '{}'", struct_name));

        assert_eq!(type_info.name(), struct_name);
        assert_eq!(type_info.size_in_bits(), 8 * mem::size_of::<T>());
        assert_eq!(type_info.size_in_bytes(), mem::size_of::<T>());
        assert_eq!(type_info.alignment(), mem::align_of::<T>());
        assert!(type_info.data.is_struct());

        let struct_info = if let TypeInfoData::Struct(s) = &type_info.data {
            s
        } else {
            panic!("Expected a struct");
        };

        assert_eq!(struct_info.num_fields(), field_names.len());
        for (lhs, rhs) in struct_info.field_names().zip(field_names) {
            assert_eq!(lhs, *rhs);
        }
        for field_type in struct_info.field_types().iter() {
            assert_eq!(field_type.guid, F::type_id().guid);
        }

        let mut offset = 0;
        for field_offset in struct_info.field_offsets() {
            assert_eq!(usize::from(*field_offset), offset);
            offset += mem::size_of::<F>();
        }

        assert_eq!(struct_info.memory_kind, memory_kind);
    }
}
