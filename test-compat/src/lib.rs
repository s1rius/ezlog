use std::path::PathBuf;

pub fn test_path() -> PathBuf {
    if cfg!(target_os = "android") {
        std::env::current_dir().unwrap()
    } else {
        dirs::cache_dir().unwrap()
    }
}
