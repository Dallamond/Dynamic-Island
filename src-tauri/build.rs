fn main() {
    tauri_build::build();
    // Con MinGW el recurso (manifiesto de Common Controls v6) solo se enlaza en los binarios;
    // sin él, los ejecutables de test fallan con STATUS_ENTRYPOINT_NOT_FOUND.
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu") {
        let out = std::env::var("OUT_DIR").unwrap();
        println!("cargo:rustc-link-arg-tests={out}/libresource.a");
    }
}
