use std::{
    fs,
    path::{Path, PathBuf},
};

#[macro_export]
macro_rules! assert_eq_text {
    ($left:expr, $right:expr) => {
        assert_eq_text!($left, $right,)
    };
    ($left:expr, $right:expr, $($tt:tt)*) => {{
        let left = $left;
        let right = $right;
        if left != right {
            if left.trim() == right.trim() {
                eprintln!("Left:\n{:?}\n\nRight:\n{:?}\n\nWhitespace difference\n", left, right);
            } else {
                let changeset = difference::Changeset::new(right, left, "\n");
                eprintln!("Left:\n{}\n\nRight:\n{}\n\nDiff:\n{}\n", left, right, changeset);
            }
            eprintln!($($tt)*);
            panic!("text differs");
        }
    }};
}

pub fn dir_tests<F>(test_data_dir: &Path, paths: &[&str], f: F)
where
    F: Fn(&str, &Path) -> String,
{
    for (path, input_code) in collect_tests(test_data_dir, paths) {
        let actual = f(&input_code, &path);
        let path = path.with_extension("txt");
        if !path.exists() {
            println!("\nfile: {}", path.display());
            println!("No .txt file with expected result, creating...\n");
            println!("{}\n{}", input_code, actual);
            fs::write(&path, &actual).unwrap();
            panic!("No expected result");
        }
        let expected = read_text(&path);
        let expected = expected.as_str();
        let actual = actual.as_str();
        assert_equal_text(expected, actual, &path);
    }
}

pub fn collect_tests(test_data_dir: &Path, paths: &[&str]) -> Vec<(PathBuf, String)> {
    paths
        .iter()
        .flat_map(|path| {
            let path = test_data_dir.to_owned().join(path);
            test_from_dir(&path).into_iter()
        })
        .map(|path| {
            let text = read_text(&path);
            (path, text)
        })
        .collect()
}

fn read_text(path: &Path) -> String {
    fs::read_to_string(path)
        .expect(&format!("File at {:?} should be valid", path))
        .replace("\r\n", "\n")
}

fn test_from_dir(dir: &Path) -> Vec<PathBuf> {
    let mut acc = Vec::new();
    for file in fs::read_dir(&dir).unwrap() {
        let file = file.unwrap();
        let path = file.path();
        if path.extension().unwrap_or_default() == "mun" {
            acc.push(path)
        }
    }
    acc.sort();
    acc
}

pub fn project_dir() -> PathBuf {
    let dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned()
}

fn assert_equal_text(expected: &str, actual: &str, path: &Path) {
    if expected == actual {
        return;
    }
    let dir = project_dir();
    let pretty_path = path.strip_prefix(&dir).unwrap_or_else(|_| path);
    if expected.trim() == actual.trim() {
        println!("whitespace difference, rewriting");
        println!("file: {}\n", pretty_path.display());
        fs::write(path, actual).unwrap();
        return;
    }
    //        println!("rewriting {}", pretty_path.display());
    //        fs::write(path, actual).unwrap();
    //        return;

    assert_eq_text!(expected, actual, "file: {}", pretty_path.display());
}
