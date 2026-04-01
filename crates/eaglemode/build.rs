fn main() {
    // Set RPATH to $ORIGIN so the binary finds plugin .so files
    // in the same directory (target/debug/ or target/release/).
    // This supplements .cargo/config.toml's LD_LIBRARY_PATH which
    // only applies to cargo-invoked commands.
    println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
}
