use std::path::PathBuf;

use specta_typescript::Typescript;

fn main() {
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src/lib/tauri");
    std::fs::create_dir_all(&out_dir).expect("Failed to create output directory");

    winservx_lib::specta_builder()
        .export(Typescript::new(), out_dir.join("bindings.ts"))
        .expect("Failed to export TypeScript bindings");
}
