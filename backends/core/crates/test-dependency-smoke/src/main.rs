fn main() {
    test_dependency_smoke::prove_dependencies().expect("portable dependency smoke");
    println!("portable dependency smoke passed");
}
