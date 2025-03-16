macro_rules! assert_snapshot_of_transpiled_fixture(
    ($fixture:literal, @$snapshot:literal) => {
        use itertools::Itertools as _;

        let mut driver = crate::integration::driver::Driver::with_fixture($fixture);

        let transpiled = driver.transpile_all_packages().unwrap();

        let formatted = transpiled
            .into_iter()
            .map(|(module_path, transpiled)| {
                let header_file = module_path.with_extension(crate::integration::driver::HEADER_EXTENSION);
                let source_file = module_path.with_extension(crate::integration::driver::SOURCE_EXTENSION);

                format!("\
//- {header_file}\n\
{}\
\n\
//- {source_file}\n\
{}",
                transpiled.header, transpiled.source)
    })
            .join("\n\n");

        insta::with_settings!({
            description => $fixture,
            omit_expression => true,
        }, {
            insta::assert_snapshot!(formatted, @$snapshot);
        });
    }
);

pub(super) use assert_snapshot_of_transpiled_fixture;
