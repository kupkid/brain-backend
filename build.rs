use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=migrations/");
    println!("cargo:rerun-if-changed=build.rs");

    let migrations_dir = Path::new("migrations");
    if !migrations_dir.exists() {
        panic!("migrations/ directory not found");
    }
}
