fn main() {
    float_view_dependency_smoke::prove_dependencies().expect("portable dependency smoke");
    println!("portable dependency smoke passed");
}
