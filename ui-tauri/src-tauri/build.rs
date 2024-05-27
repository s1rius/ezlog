fn main() {
    println!("cargo:rerun-if-changed=dist");

    // create dist dir, to make clippy happy
    std::fs::create_dir_all("../dist").expect("create dist fail");

    tauri_build::build()
}
