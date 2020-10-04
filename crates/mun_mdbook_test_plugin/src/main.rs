use mdbook::renderer::RenderContext;
use mdbook::BookItem;

use std::io;

use regex::Regex;

use mun_test::{CompileAndRunTestDriver, CompileTestDriver};

use mun_runtime::{invoke_fn, RuntimeBuilder};

use std::panic;

enum TestingMethod {
    Ignore,
    CompileFail,
    Compile,
    CompileAndRun,
}

impl Default for TestingMethod {
    fn default() -> Self {
        TestingMethod::CompileAndRun
    }
}

fn get_testing_method(code: &str) -> TestingMethod {
    let mut testing_method = TestingMethod::default();

    for line in code.lines() {
        if let Some(first_char) = line.chars().next() {
            if first_char != ',' {
                return testing_method;
            }
        } else {
            return testing_method;
        }
        if line == ",compile_fail" {
            return TestingMethod::CompileFail;
        }
        if line == ",no_run" {
            testing_method = TestingMethod::Compile;
        }
        if line == ",ignore" {
            return TestingMethod::Ignore;
        }
    }

    testing_method
}

fn test_code(code: &str) {
    let testing_method = get_testing_method(code);

    // Removing '#' from code
    let mut code = code.replace("\n#", "\n");

    // Removing testing commands
    for testing_command in ["compile_fail", "no_run", "ignore"].iter() {
        code = code.replace(format!(",{}", testing_command).as_str(), "");
    }

    fn config_fn(runtime_builder: RuntimeBuilder) -> RuntimeBuilder {
        runtime_builder
    }

    match testing_method {
        TestingMethod::Ignore => {}
        TestingMethod::CompileFail => {
            let previous_hook = panic::take_hook();
            panic::set_hook(Box::new(|_| {}));

            if panic::catch_unwind(|| CompileTestDriver::new(&code)).is_ok() {
                panic::set_hook(previous_hook);
                panic!("Code that should have caused the error compiled successfully ❌")
            }

            panic::set_hook(previous_hook);
        }
        TestingMethod::Compile => {
            CompileTestDriver::new(&code);
        }
        TestingMethod::CompileAndRun => {
            let compile_and_run_test_driver =
                CompileAndRunTestDriver::new(&code, config_fn).unwrap();

            if compile_and_run_test_driver
                .runtime()
                .borrow()
                .get_function_definition("main")
                .is_none()
            {
                panic!("Function `main` not found in mun code, but requested.");
            }

            let _: () =
                invoke_fn!(compile_and_run_test_driver.runtime().borrow_mut(), "main").unwrap();
        }
    }
}

fn main() {
    let mut stdin = io::stdin();

    let ctx = RenderContext::from_json(&mut stdin).unwrap();

    let re = Regex::new(r"```mun((?s:.)*?)```").unwrap();

    for item in ctx.book.iter() {
        if let BookItem::Chapter(ref ch) = *item {
            println!("Testing code in chapter '{}':", ch.name);
            for capture in re.captures_iter(&ch.content) {
                test_code(&capture[1]);
            }
            println!("Pass ✔");
        }
    }
}
