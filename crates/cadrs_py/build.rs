//! A Python extension leaves libpython's symbols to the interpreter that loads it. macOS's linker
//! refuses undefined symbols in a dylib unless told `-undefined dynamic_lookup`, which pyo3's
//! `extension-module` doesn't pass by itself (no-op on other platforms).
fn main() {
    pyo3_build_config::add_extension_module_link_args();
}
