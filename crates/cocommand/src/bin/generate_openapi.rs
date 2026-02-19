//! Writes the OpenAPI spec to `packages/api-client/openapi.json`.
//!
//! Usage: `cargo run --bin generate_openapi`

use cocommand::server::openapi::ApiDoc;
use utoipa::OpenApi;

fn main() {
    let spec = ApiDoc::openapi()
        .to_pretty_json()
        .expect("failed to serialize OpenAPI spec");

    let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../packages/api-client");

    std::fs::create_dir_all(&out_dir).expect("failed to create output directory");

    let out_path = out_dir.join("openapi.json");
    std::fs::write(&out_path, &spec).expect("failed to write openapi.json");

    println!("Wrote OpenAPI spec to {}", out_path.display());
}
