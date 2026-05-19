#[cfg(all(feature = "ratzilla-ui", target_arch = "wasm32"))]
fn main() -> std::io::Result<()> {
    tensor_vm_explorer::ratzilla_ui::run()
}

#[cfg(not(all(feature = "ratzilla-ui", target_arch = "wasm32")))]
fn main() {
    eprintln!(
        "tensorvm-explorer-ratzilla requires --target wasm32-unknown-unknown --features ratzilla-ui"
    );
}
