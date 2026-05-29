fn main() {
    tauri_build::build()

    // TypeScript bindings are generated at runtime via tauri-specta
    // To regenerate, run: cargo build && the bindings are written to src/lib/bindings.ts
}
