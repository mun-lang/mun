use mun_target::abi::TargetDataLayout;
use mun_target::spec::Target;

#[test]
fn data_layout_windows() {
    let layout =
        TargetDataLayout::parse(&Target::search("x86_64-pc-windows-msvc").unwrap()).unwrap();

    insta::assert_debug_snapshot!(layout);
}

#[test]
fn data_layout_darwin() {
    let layout = TargetDataLayout::parse(&Target::search("x86_64-apple-darwin").unwrap()).unwrap();

    insta::assert_debug_snapshot!(layout);
}

#[test]
fn data_layout_linux() {
    let layout =
        TargetDataLayout::parse(&Target::search("x86_64-unknown-linux-gnu").unwrap()).unwrap();

    insta::assert_debug_snapshot!(layout);
}
