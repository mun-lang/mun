//! A crate to generate and run Mun tests based on mdbook content. It is based on
//! [mdbook-test](https://github.com/Michael-F-Bryan/mdbook-test) and
//! [rust-skeptic](https://github.com/budziq/rust-skeptic).

#![warn(missing_docs)]

pub mod runtime;

use mdbook::renderer::RenderContext;
use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag};
use std::{
    cell::RefCell,
    env,
    fs::File,
    io,
    io::{Read, Write},
    mem,
    path::{Path, PathBuf},
};

#[derive(Default)]
struct BookStore {
    book: RefCell<mdbook::book::Book>,
}

impl mdbook::renderer::Renderer for BookStore {
    fn name(&self) -> &str {
        "mdbook_skeptic"
    }

    fn render(&self, ctx: &RenderContext) -> mdbook::errors::Result<()> {
        *self.book.borrow_mut() = ctx.book.clone();
        Ok(())
    }
}

/// Generates tests for an mdbook with the specified root.
pub fn generate_doc_tests_from_mdbook(book_root: impl Into<PathBuf>) {
    // Load the mdbook at the specified location
    let (src_dir, book) = {
        let book = mdbook::book::MDBook::load(book_root).expect("could not load mdbook");
        let book_store = BookStore::default();
        book.execute_build_process(&book_store)
            .expect("error building mdbook");
        (book.source_dir(), book_store.book.take())
    };

    // Inform cargo that it needs to rerun the build script if one of the source files of the book
    // changed.
    for item in book.iter() {
        if let mdbook::BookItem::Chapter(ch) = item {
            if let Some(path) = &ch.path {
                println!("cargo:rerun-if-changed={}", src_dir.join(path).display());
            }
        }
    }

    // Extract all the tests from the book.
    let tests = book
        .iter()
        .filter_map(|item| match item {
            mdbook::BookItem::Chapter(cp) => Some(cp),
            _ => None,
        })
        .flat_map(|cp| {
            extract_tests_from_string(
                &cp.content,
                &cp.path
                    .as_ref()
                    .and_then(|p| {
                        p.file_stem()
                            .and_then(|file_stem| file_stem.to_str())
                            .map(ToString::to_string)
                    })
                    .map(|file_stem| sanitize_test_name(&file_stem))
                    .unwrap_or_else(String::new),
            )
        })
        .collect::<Vec<_>>();

    // Get information from environment variables
    let out_dir = env::var("OUT_DIR").expect("could not determine output directory");

    // Determine the output location of the test files
    let mut out_file = PathBuf::from(out_dir);
    out_file.push("skeptic-tests.rs");

    // Emit all tests
    emit_tests(out_file, tests);
}

/// Describes a single test extract from the documentation
#[derive(Default, Clone, Eq, PartialEq, Debug)]
struct Test {
    name: String,
    text: Vec<String>,
    ignore: bool,
    no_run: bool,
    compile_fail: bool,
}

/// Extracts all the Mun tests from the specified markdown. The `file_stem` is appended to the name
/// of the tests.
fn extract_tests_from_string(markdown: &str, file_stem: &str) -> Vec<Test> {
    enum Block {
        None,
        Code(Vec<String>),
        Header(String),
    }

    let mut tests = Vec::new();
    let mut block = Block::None;
    let mut section = None;
    let mut code_block_start = 0;

    for (event, range) in Parser::new(markdown).into_offset_iter() {
        let line_number = bytecount::count(&markdown.as_bytes()[0..range.end], b'\n');
        match event {
            Event::Start(Tag::Heading(level)) if level < 3 => {
                block = Block::Header(String::new());
            }
            Event::End(Tag::Heading(level)) if level < 3 => {
                let cur_buffer = mem::replace(&mut block, Block::None);
                if let Block::Header(sect) = cur_buffer {
                    section = Some(sanitize_test_name(&sect));
                }
            }
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(ref info))) => {
                let code_block_info = parse_code_block_info(info);
                if code_block_info.is_mun {
                    block = Block::Code(Vec::new());
                }
            }
            Event::End(Tag::CodeBlock(CodeBlockKind::Fenced(ref info))) => {
                let code_block_info = parse_code_block_info(info);
                if let Block::Code(buf) = mem::replace(&mut block, Block::None) {
                    let name = if let Some(ref section) = section {
                        format!("{}_sect_{}_line_{}", file_stem, section, code_block_start)
                    } else {
                        format!("{}_line_{}", file_stem, code_block_start)
                    };
                    tests.push(Test {
                        name,
                        text: buf,
                        ignore: code_block_info.ignore,
                        no_run: code_block_info.no_run,
                        compile_fail: code_block_info.compile_fail,
                    });
                }
            }
            Event::Text(text) => {
                if let Block::Code(ref mut buf) = block {
                    if buf.is_empty() {
                        code_block_start = line_number;
                    }
                    buf.push(text.into_string());
                } else if let Block::Header(ref mut buf) = block {
                    buf.push_str(&*text);
                }
            }
            _ => (),
        }
    }

    tests
}

/// Given a test name convert it to a valid identifier.
fn sanitize_test_name(name: &str) -> String {
    name.to_ascii_lowercase()
        .chars()
        .map(|ch| {
            if ch.is_ascii() && ch.is_alphanumeric() {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
        .split('_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

/// Describes information extracted from code-blocks.
struct CodeBlockInfo {
    is_mun: bool,
    ignore: bool,
    no_run: bool,
    compile_fail: bool,
}

/// Parses the code following code block tags for valid mun tokens. These tokens are similar to
/// rustdoc.
fn parse_code_block_info(info: &str) -> CodeBlockInfo {
    let tokens = info.split(|c: char| !(c == '_' || c == '-' || c.is_alphanumeric()));

    let mut seen_mun_tags = false;
    let mut seen_other_tags = false;

    let mut info = CodeBlockInfo {
        is_mun: false,
        ignore: false,
        no_run: false,
        compile_fail: false,
    };

    for token in tokens {
        match token {
            "" => {}
            "mun" => {
                info.is_mun = true;
                seen_mun_tags = true
            }
            "ignore" => {
                info.ignore = true;
                seen_mun_tags = true
            }
            "no_run" => {
                info.no_run = true;
                seen_mun_tags = true;
            }
            "compile_fail" => {
                info.compile_fail = true;
                seen_mun_tags = true;
            }
            _ => seen_other_tags = true,
        }
    }

    info.is_mun &= !seen_other_tags || seen_mun_tags;
    info
}

/// Emit all test cases to the specified file
fn emit_tests(out_path: impl AsRef<Path>, tests: Vec<Test>) {
    let mut content = String::new();

    // All tests need the api from mun_skeptic::runtime
    content.push_str("extern crate mun_skeptic;\n");

    for test in tests.iter() {
        let test_string = emit_test_runner(test).unwrap();
        content.push_str(&test_string);
    }

    write_if_contents_changed(out_path.as_ref(), &content).unwrap();
}

/// Creates the Mun code that this test will be operating on.
fn create_test_input(lines: &[String]) -> String {
    // Build the text of the code by stripping any leading #
    itertools::Itertools::intersperse(
        lines
            .iter()
            .flat_map(|text| text.lines())
            .map(|line| line.strip_prefix('#').unwrap_or(line)),
        "\n",
    )
    .collect()
}

/// Emit code to run test specified test
fn emit_test_runner(test: &Test) -> io::Result<String> {
    let test_text = create_test_input(&test.text);
    let mut s: Vec<u8> = Vec::new();
    if test.ignore {
        writeln!(s, "#[ignore]")?;
    }
    writeln!(s, "#[test] fn {}() {{", test.name)?;
    writeln!(s, "    let s = &r####\"\n{}\"####;", test_text)?;

    let mode = match (test.no_run, test.compile_fail) {
        (_, true) => "ShouldNotCompile",
        (true, _) => "Compile",
        (false, _) => "CompileAndRun",
    };

    writeln!(
        s,
        "    mun_skeptic::runtime::run_test(\n        s,\n        mun_skeptic::runtime::TestMode::{});",
        mode
    )?;
    writeln!(s, "}}")?;
    writeln!(s)?;

    Ok(String::from_utf8(s).unwrap())
}

/// Write the contents of the specified path but only if the contents is different. This ensures
/// that a filesystem write event is only emitted when the content actually changes.
fn write_if_contents_changed(name: &Path, contents: &str) -> io::Result<()> {
    // Can't open in write mode now as that would modify the last changed timestamp of the file
    match File::open(name) {
        Ok(mut file) => {
            let mut current_contents = String::new();
            file.read_to_string(&mut current_contents)?;
            if current_contents == contents {
                // No change avoid writing to avoid updating the timestamp of the file
                return Ok(());
            }
        }
        Err(ref err) if err.kind() == io::ErrorKind::NotFound => (),
        Err(err) => return Err(err),
    }
    let mut file = File::create(name)?;
    file.write_all(contents.as_bytes())?;
    Ok(())
}
