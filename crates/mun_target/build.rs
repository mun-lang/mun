fn main() {
    println!(
        "cargo:rustc-env=CFG_COMPILER_HOST_TRIPLE={}",
        std::env::var("TARGET").unwrap()
    );
}
