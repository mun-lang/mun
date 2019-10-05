///! This library contains the code required to go from source code to binaries.
mod diagnostic;

use crate::diagnostic::Emit;
use failure::Error;
use mun_codegen::IrDatabase;
use mun_errors::{Diagnostic, Level};
use mun_hir::diagnostics::{Diagnostic as HirDiagnostic, DiagnosticSink};
use mun_hir::{salsa, FileId, HirDisplay, Module, PackageInput, RelativePathBuf, SourceDatabase};
use mun_syntax::ast::AstNode;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use termcolor::{ColorChoice, StandardStream};

pub use mun_codegen::OptimizationLevel;
use mun_syntax::{ast, SyntaxKind};
use mun_target::spec;

#[derive(Debug, Clone)]
pub struct CompilerOptions {
    pub input: PathBuf,
    pub target: Option<String>,
    pub optimization_lvl: OptimizationLevel,
}

#[salsa::database(
    mun_hir::SourceDatabaseStorage,
    mun_hir::DefDatabaseStorage,
    mun_hir::HirDatabaseStorage,
    mun_codegen::IrDatabaseStorage
)]
#[derive(Debug)]
pub struct CompilerDatabase {
    events: Mutex<Option<Vec<salsa::Event<CompilerDatabase>>>>,
    runtime: salsa::Runtime<CompilerDatabase>,
}

impl salsa::Database for CompilerDatabase {
    fn salsa_runtime(&self) -> &salsa::Runtime<CompilerDatabase> {
        &self.runtime
    }
    fn salsa_event(&self, event: impl Fn() -> salsa::Event<CompilerDatabase>) {
        let mut events = self.events.lock().unwrap();
        if let Some(events) = &mut *events {
            events.push(event());
        }
    }
}

/// Implements the ability to retrieve query results in a closure.
impl CompilerDatabase {
    pub fn log(&self, f: impl FnOnce()) -> Vec<salsa::Event<CompilerDatabase>> {
        *self.events.lock().unwrap() = Some(Vec::new());
        f();
        self.events.lock().unwrap().take().unwrap()
    }

    pub fn log_executed(&self, f: impl FnOnce()) -> Vec<String> {
        let events = self.log(f);
        events
            .into_iter()
            .filter_map(|e| match e.kind {
                // This pretty horrible, but `Debug` is the only way to inspect
                // QueryDescriptor at the moment.
                salsa::EventKind::WillExecute { database_key } => {
                    Some(format!("{:#?}", database_key.kind))
                }
                _ => None,
            })
            .collect()
    }
}

impl CompilerDatabase {
    fn from_file(path: &Path) -> Result<(CompilerDatabase, FileId), Error> {
        let mut db = CompilerDatabase {
            runtime: salsa::Runtime::default(),
            events: Mutex::new(Some(Vec::new())),
        };
        let file_id = FileId(0);
        db.set_file_relative_path(file_id, RelativePathBuf::from_path(path).unwrap());
        db.set_file_text(file_id, Arc::new(std::fs::read_to_string(path)?));
        let mut package_input = PackageInput::default();
        package_input.add_module(file_id);
        db.set_package_input(Arc::new(package_input));
        db.set_optimization_lvl(OptimizationLevel::Default);
        db.set_target(mun_target::spec::Target::search(host_triple()).unwrap());

        let context = mun_codegen::Context::create();
        db.set_context(Arc::new(context));

        Ok((db, file_id))
    }
}

pub fn host_triple() -> &'static str {
    // Get the host triple out of the build environment. This ensures that our
    // idea of the host triple is the same as for the set of libraries we've
    // actually built.  We can't just take LLVM's host triple because they
    // normalize all ix86 architectures to i386.
    //
    // Instead of grabbing the host triple (for the current host), we grab (at
    // compile time) the target triple that this rustc is built with and
    // calling that (at runtime) the host triple.
    (option_env!("CFG_COMPILER_HOST_TRIPLE")).expect("CFG_COMPILER_HOST_TRIPLE")
}

fn diagnostics(db: &CompilerDatabase, file_id: FileId) -> Vec<Diagnostic> {
    let parse = db.parse(file_id);
    let mut result = Vec::new();

    result.extend(parse.errors().iter().map(|err| Diagnostic {
        level: Level::Error,
        loc: err.location(),
        message: format!("Syntax Error: {}", err),
    }));

    let result = RefCell::new(result);
    let mut sink = DiagnosticSink::new(|d| {
        result.borrow_mut().push(Diagnostic {
            level: Level::Error,
            loc: d.highlight_range().into(),
            message: d.message(),
        });
    })
    .on::<mun_hir::diagnostics::UnresolvedValue, _>(|d| {
        let text = d.expr.to_node(&parse.tree().syntax()).text().to_string();
        result.borrow_mut().push(Diagnostic {
            level: Level::Error,
            loc: d.highlight_range().into(),
            message: format!("could not find value `{}` in this scope", text),
        });
    })
    .on::<mun_hir::diagnostics::UnresolvedType, _>(|d| {
        let text = d
            .type_ref
            .to_node(&parse.tree().syntax())
            .syntax()
            .text()
            .to_string();
        result.borrow_mut().push(Diagnostic {
            level: Level::Error,
            loc: d.highlight_range().into(),
            message: format!("could not find type `{}` in this scope", text),
        });
    })
    .on::<mun_hir::diagnostics::MismatchedType, _>(|d| {
        result.borrow_mut().push(Diagnostic {
            level: Level::Error,
            loc: d.highlight_range().into(),
            message: format!(
                "expected `{}`, found `{}`",
                d.expected.display(db),
                d.found.display(db)
            ),
        });
    })
    .on::<mun_hir::diagnostics::DuplicateDefinition, _>(|d| {
        result.borrow_mut().push(Diagnostic {
            level: Level::Error,
            loc: match d.definition.kind() {
                SyntaxKind::FUNCTION_DEF => {
                    ast::FunctionDef::cast(d.definition.to_node(&parse.tree().syntax()))
                        .map(|f| f.signature_range())
                        .unwrap_or(d.highlight_range())
                        .into()
                }
                _ => d.highlight_range().into(),
            },
            message: d.message(),
        });
    });

    if let Some(module) = Module::package_modules(db)
        .iter()
        .find(|m| m.file_id() == file_id)
    {
        module.diagnostics(db, &mut sink)
    }

    drop(sink);
    result.into_inner()
}

pub fn main(options: CompilerOptions) -> Result<(), failure::Error> {
    let (mut db, file_id) = CompilerDatabase::from_file(&options.input)?;
    db.set_optimization_lvl(options.optimization_lvl);
    if let Some(target) = options.target {
        db.set_target(spec::Target::search(&target).unwrap());
    }

    let diagnostics = diagnostics(&db, file_id);
    if !diagnostics.is_empty() {
        let mut writer = StandardStream::stderr(ColorChoice::Auto);
        for diagnostic in diagnostics {
            diagnostic.emit(&mut writer, &db, file_id)?;
        }
        return Ok(());
    }

    //let module = db.module_ir(file_id);
    //println!("{}", module.llvm_module.print_to_string().to_string());

    mun_codegen::write_module_shared_object(&db, file_id);

    Ok(())
}
