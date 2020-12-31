use crate::{
    db::DefDatabase, fixture::WithFixture, ids::ItemDefinitionId, mock::MockDatabase,
    package_defs::PackageDefs, DiagnosticSink, Function, HirDatabase, Module, Package, Struct,
    TypeAlias,
};
use itertools::Itertools;
use rustc_hash::FxHashSet;

#[test]
fn use_alias() {
    resolve_snapshot(
        r#"
    //- /foo.mun
    pub struct Ok;

    //- /bar.mun
    pub use package::foo::Ok as ReallyOk;

    pub struct Ok;

    //- /baz.mun
    use package::bar::ReallyOk;
    "#,
    )
}

#[test]
fn use_duplicate_name() {
    resolve_snapshot(
        r#"
    //- /foo.mun
    pub struct Ok;

    //- /bar.mun
    use package::foo::Ok;

    pub struct Ok;
    "#,
    )
}

#[test]
fn use_cyclic_wildcard() {
    resolve_snapshot(
        r#"
    //- /foo.mun
    pub use super::baz::*;

    pub struct Ok;

    //- /baz.mun
    pub use super::foo::{self, *};
    "#,
    )
}

#[test]
fn use_wildcard() {
    resolve_snapshot(
        r#"
    //- /foo.mun
    pub struct Foo;

    //- /foo/bar.mun
    pub use super::Foo;
    pub struct FooBar;

    //- /bar.mun
    use package::foo::bar::*;   // Should reference two structs (Foo and FooBar)
    "#,
    )
}

#[test]
fn use_self() {
    resolve_snapshot(
        r#"
    //- /foo.mun
    pub struct Ok;

    //- /bar.mun
    use super::foo::{self};
    use foo::Ok;
    "#,
    )
}

#[test]
fn use_cyclic() {
    resolve_snapshot(
        r#"
    //- /foo.mun
    use super::baz::Cyclic;

    pub struct Ok;

    //- /bar.mun
    use super::foo::{Cyclic, Ok};

    //- /baz.mun
    use super::bar::{Cyclic, Ok};
    "#,
    )
}

#[test]
fn use_unresolved() {
    resolve_snapshot(
        r#"
    //- /foo.mun
    pub struct Foo;

    //- /mod.mun
    use foo::Foo;   // works
    use foo::Bar;   // doesnt work (Bar does not exist)
    use baz::Baz;   // doesnt work (baz does not exist)
    "#,
    )
}

#[test]
fn use_() {
    resolve_snapshot(
        r#"
    //- /bar.mun
    use package::Foo;
    pub struct Bar(Foo);

    //- /mod.mun
    pub use foo::Foo; // Re-export a child's definition

    struct Baz;

    //- /foo.mun
    use package::{bar::Bar, Baz};

    pub struct Foo {
        baz: Baz, // Can use private definitions from any of its ancestors
    }

    pub fn foo_from_bar(bar: Bar) -> Foo {
        bar.0
    }
    "#,
    )
}

fn resolve_snapshot(text: &str) {
    let text = text.trim().replace("\n    ", "\n");
    let resolved = resolve(&text);
    insta::assert_snapshot!(insta::_macro_support::AutoName, resolved.trim(), &text);
}

fn resolve(content: &str) -> String {
    let db = MockDatabase::with_files(content);

    Package::all(&db)
        .iter()
        .map(|package| {
            let package_defs = db.package_defs(package.id);
            tree_for_module(&db, &package_defs, package.root_module(&db)).to_string()
        })
        .intersperse("\n".to_owned())
        .collect()
}

fn tree_for_module(
    db: &dyn HirDatabase,
    package_defs: &PackageDefs,
    module: Module,
) -> text_trees::StringTreeNode {
    // Construct a tree node
    let mut node = text_trees::StringTreeNode::new(format!(
        "mod {}",
        module
            .name(db)
            .map(|name| name.to_string())
            .unwrap_or_else(|| "mod".to_owned())
    ));

    // Add module level diagnostics
    let mut diag_sink = DiagnosticSink::new(|diag| {
        node.push(format!(
            "ERROR: {}: {}",
            diag.highlight_range(),
            diag.message()
        ));
    });
    module.diagnostics(db, &mut diag_sink);
    drop(diag_sink);

    // Iterate over all declarations and add them as nodes
    let scope = &package_defs[module.id.local_id];
    let local_declarations = scope.declarations().collect::<FxHashSet<_>>();
    let used_declarations = scope
        .entries()
        .filter_map(|entry| entry.1.take_types().map(|(def, _)| def))
        .collect::<Vec<_>>();
    for def in local_declarations.iter().chain(
        used_declarations
            .iter()
            .filter(|decl| !local_declarations.contains(*decl)),
    ) {
        let is_local = local_declarations.contains(&def);
        match def {
            ItemDefinitionId::ModuleId(m) => {
                if m.package == module.id.package
                    && module
                        .children(db)
                        .into_iter()
                        .find(|child_id| child_id.id == *m)
                        .is_none()
                {
                    let module: Module = (*m).into();
                    node.push(format!(
                        "use mod {}",
                        fully_qualified_module_path(db, module)
                    ));
                }
            }
            ItemDefinitionId::FunctionId(f) => {
                let func: Function = (*f).into();
                let name = func.name(db);
                if is_local {
                    node.push(format!("fn {}", name));
                } else {
                    let fully_qualified_name = format!(
                        "{}::{}",
                        fully_qualified_module_path(db, func.module(db)),
                        name
                    );
                    node.push(format!("use fn {}", fully_qualified_name));
                }
            }
            ItemDefinitionId::StructId(s) => {
                let strukt: Struct = (*s).into();
                let name = strukt.name(db);
                if is_local {
                    node.push(format!("struct {}", name));
                } else {
                    let fully_qualified_name = format!(
                        "{}::{}",
                        fully_qualified_module_path(db, strukt.module(db)),
                        name
                    );
                    node.push(format!("use struct {}", fully_qualified_name));
                }
            }
            ItemDefinitionId::TypeAliasId(alias) => {
                let alias: TypeAlias = (*alias).into();
                let name = alias.name(db);
                if is_local {
                    node.push(format!("type {}", name));
                } else {
                    let fully_qualified_name = format!(
                        "{}::{}",
                        fully_qualified_module_path(db, alias.module(db)),
                        name
                    );
                    node.push(format!("use type {}", fully_qualified_name));
                }
            }
            ItemDefinitionId::PrimitiveType(_) => {}
        }
    }

    // Iterate over all children of this module
    for child_module in module.children(db) {
        node.push_node(tree_for_module(db, package_defs, child_module))
    }

    node
}

/// Returns a fully qualified path of a module e.g. `package::foo::bar::baz`
fn fully_qualified_module_path(db: &dyn HirDatabase, module: Module) -> String {
    module
        .path_to_root(db)
        .into_iter()
        .map(|m| {
            m.name(db)
                .map(|name| name.to_string())
                .unwrap_or_else(|| "package".to_owned())
        })
        .rev()
        .intersperse("::".to_string())
        .collect::<String>()
}
