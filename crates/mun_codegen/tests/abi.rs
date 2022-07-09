use libloader::MunLibrary;
use mun_test::CompileTestDriver;

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
    pub fn {fn_name2}() {{
        let a = {struct_name}(1.0, 2.0);
        let b = [1,2,3]
    }}

    pub struct {struct_name}(f64, f64);
    pub struct(value) {struct_name2} {{ a: i32, b: i32 }};
    "#,
    ));

    // Assert that all library functions are exposed
    // Safety: We compiled the code ourselves, therefor loading the library is safe
    let lib = unsafe { MunLibrary::new(driver.lib_path()) }
        .expect("Failed to load generated Mun library.");

    assert_eq!(abi::ABI_VERSION, unsafe { lib.get_abi_version() });
    insta::assert_ron_snapshot!(unsafe { lib.get_info() },
    @r###"
    AssemblyInfo(
      symbols: ModuleInfo(
        path: "",
        functions: [
          FunctionDefinition(
            prototype: FunctionPrototype(
              name: "bar",
              signature: FunctionSignature(
                arg_types: [],
                return_type: None,
              ),
            ),
          ),
          FunctionDefinition(
            prototype: FunctionPrototype(
              name: "foo",
              signature: FunctionSignature(
                arg_types: [
                  Concrete("60db469c-3f59-4a25-47ad-349fd5922541"),
                ],
                return_type: Some(Concrete("17797a74-19d6-3217-d235-954317885bfa")),
              ),
            ),
          ),
        ],
        types: [
          TypeDefinition(
            name: "Bar",
            size_in_bits: 64,
            alignment: 4,
            data: Struct(StructInfo(
              guid: "69b56992-5a33-7f3e-c3e5-a5c8deb1bc4e",
              fields: [
                Field(
                  name: "a",
                  type: Concrete("17797a74-19d6-3217-d235-954317885bfa"),
                  offset: 0,
                ),
                Field(
                  name: "b",
                  type: Concrete("17797a74-19d6-3217-d235-954317885bfa"),
                  offset: 4,
                ),
              ],
              memory_kind: Value,
            )),
          ),
          TypeDefinition(
            name: "Foo",
            size_in_bits: 128,
            alignment: 8,
            data: Struct(StructInfo(
              guid: "9fe596c7-9975-0310-0084-6f4e4d5b7be2",
              fields: [
                Field(
                  name: "0",
                  type: Concrete("60db469c-3f59-4a25-47ad-349fd5922541"),
                  offset: 0,
                ),
                Field(
                  name: "1",
                  type: Concrete("60db469c-3f59-4a25-47ad-349fd5922541"),
                  offset: 8,
                ),
              ],
              memory_kind: Gc,
            )),
          ),
        ],
      ),
      dispatch_table: DispatchTable(
        prototypes: [
          FunctionPrototype(
            name: "new",
            signature: FunctionSignature(
              arg_types: [
                Pointer(PointerTypeId(
                  pointee: Concrete("af39d38b-abb4-d6f6-4a2e-5cffe78b0981"),
                  mutable: false,
                )),
                Pointer(PointerTypeId(
                  pointee: Concrete("af39d38b-abb4-d6f6-4a2e-5cffe78b0981"),
                  mutable: true,
                )),
              ],
              return_type: Some(Pointer(PointerTypeId(
                pointee: Pointer(PointerTypeId(
                  pointee: Concrete("af39d38b-abb4-d6f6-4a2e-5cffe78b0981"),
                  mutable: true,
                )),
                mutable: false,
              ))),
            ),
          ),
          FunctionPrototype(
            name: "new_array",
            signature: FunctionSignature(
              arg_types: [
                Pointer(PointerTypeId(
                  pointee: Concrete("af39d38b-abb4-d6f6-4a2e-5cffe78b0981"),
                  mutable: false,
                )),
                Concrete("a6e76720-d18b-1a71-601f-1e07bb354071"),
                Pointer(PointerTypeId(
                  pointee: Concrete("af39d38b-abb4-d6f6-4a2e-5cffe78b0981"),
                  mutable: true,
                )),
              ],
              return_type: Some(Pointer(PointerTypeId(
                pointee: Pointer(PointerTypeId(
                  pointee: Concrete("af39d38b-abb4-d6f6-4a2e-5cffe78b0981"),
                  mutable: true,
                )),
                mutable: false,
              ))),
            ),
          ),
        ],
      ),
      type_lut: [
        Elem(
          name: "Bar",
          type: Concrete("69b56992-5a33-7f3e-c3e5-a5c8deb1bc4e"),
        ),
        Elem(
          name: "Foo",
          type: Concrete("9fe596c7-9975-0310-0084-6f4e4d5b7be2"),
        ),
        Elem(
          name: "[core::i32]",
          type: Array(ArrayTypeId(
            element: Concrete("17797a74-19d6-3217-d235-954317885bfa"),
          )),
        ),
        Elem(
          name: "core::f64",
          type: Concrete("60db469c-3f59-4a25-47ad-349fd5922541"),
        ),
        Elem(
          name: "core::i32",
          type: Concrete("17797a74-19d6-3217-d235-954317885bfa"),
        ),
      ],
      dependencies: [],
    )
    "###);
}
