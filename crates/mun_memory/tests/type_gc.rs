use mun_memory::{HasStaticType, StructTypeBuilder, Type};

#[test]
fn test_type_collection() {
    // Initially there is no type data
    let current_stats = Type::collect_unreferenced_type_data();
    assert_eq!(current_stats.collected_types, 0);
    assert_eq!(current_stats.remaining_types, 0);

    // Lets get some static type information
    let _ = i32::type_info();
    let _ = <()>::type_info();
    let _ = bool::type_info();

    // The static types should not be collected because they are static.
    let current_stats = Type::collect_unreferenced_type_data();
    assert_eq!(current_stats.collected_types, 0);
    assert_eq!(current_stats.remaining_types, 3);

    // Lets create a custom type that references one of these static types.
    let foo_type = StructTypeBuilder::new("Foo")
        .add_field("bar", i32::type_info().clone())
        .finish();

    // Now one type should be collected because we have a reference to it, one is added
    let current_stats = Type::collect_unreferenced_type_data();
    assert_eq!(current_stats.collected_types, 0);
    assert_eq!(current_stats.remaining_types, 4);

    // Drop the type and see if collection worked
    drop(foo_type);
    let current_stats = Type::collect_unreferenced_type_data();
    assert_eq!(current_stats.collected_types, 1);
    assert_eq!(current_stats.remaining_types, 3);
}
