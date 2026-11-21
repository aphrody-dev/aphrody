//! Generates `include/aphrody_av.h` from the `ffi` module with cbindgen.
fn main() {
    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
    let output = std::env::args()
        .nth(1)
        .unwrap_or_else(|| format!("{crate_dir}/include/aphrody_av.h"));
    match cbindgen::generate(&crate_dir) {
        Ok(bindings) => {
            bindings.write_to_file(&output);
            println!("wrote {output}");
        }
        Err(error) => {
            eprintln!("cbindgen failed: {error}");
            std::process::exit(1);
        }
    }
}
