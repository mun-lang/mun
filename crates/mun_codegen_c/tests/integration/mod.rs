mod config;
mod db;
mod driver;
mod snapshot;

use snapshot::assert_snapshot_of_transpiled_fixture;

#[test]
fn end_to_end() {
    assert_snapshot_of_transpiled_fixture!("\
//- /src/mod.mun
pub fn main() -> i32 { foo::foo() }

fn bar() -> i32 { 5 }

//- /src/foo.mun
pub fn foo() -> i32 { super::bar() }", @""
    );
}

#[test]
fn structure() {
    assert_snapshot_of_transpiled_fixture!("\
//- /src/mod.mun
pub fn main() -> i32 { foo::foo().x + bar::bar().y + baz() }

fn baz() -> i32 { 5 }

//- /src/foo.mun
pub struct(gc) Foo {
    pub x: i32,
}

pub fn foo() -> Foo {
    Foo { x: 1 }
}

//- /src/bar.mun
pub struct(value) Bar {
    pub y: i32,
}

pub fn bar() -> Bar {
    Bar { y: 2 }
}", @""
    );
}

#[test]
fn recursive_struct() {
    assert_snapshot_of_transpiled_fixture!("\
//- /src/mod.mun
pub fn main() -> i32 { foo::new().inner.value + bar::new().inner.value + baz() }

fn baz() -> i32 { 5 }

//- /src/foo.mun
pub struct(gc) Outer {
    pub inner: Inner,
}

pub struct(value) Inner {
    pub value: i32,
}

pub fn new() -> Outer {
    Outer { inner: Inner { value: 1 } }
}

//- /src/bar.mun
pub struct(value) Outer {
    pub inner: Inner,
}

pub struct(gc) Inner {
    pub value: i32,
}

pub fn new() -> Outer {
    Outer { inner: Inner { value: 1 } }
}", @""
    );
}
