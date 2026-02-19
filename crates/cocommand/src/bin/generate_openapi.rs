//! Writes the OpenAPI spec to `packages/api/openapi.json`.
//!
//! Usage: `cargo run --bin generate_openapi`

use cocommand::server::openapi::generate_full_spec;

fn main() {
    let spec = generate_full_spec();

    let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packages/api");

    std::fs::create_dir_all(&out_dir).expect("failed to create output directory");

    let out_path = out_dir.join("openapi.json");
    std::fs::write(&out_path, &spec).expect("failed to write openapi.json");

    println!("Wrote OpenAPI spec to {}", out_path.display());
}
