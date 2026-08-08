fn main() {
    let (version, build) = host_runtime::identity();
    assert_eq!(version, 1);
    assert!(!build.is_empty());
    println!("float-view-core abi={version} build={build}");
}
