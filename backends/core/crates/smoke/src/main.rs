fn main() {
    let (version, build) = float_view_runtime::identity();
    assert_eq!(version, 1);
    assert!(!build.is_empty());
    println!("float-view-core abi={version} build={build}");
}
