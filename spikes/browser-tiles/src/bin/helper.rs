//! The CEF helper executable on macOS: every Chromium subprocess (renderer, GPU, network,
//! storage) runs this. Linux runs subprocesses from the main binary instead.
#[cfg(target_os = "macos")]
fn main() {
    use cef::{args::Args, *};
    let loader = library_loader::LibraryLoader::new(&std::env::current_exe().unwrap(), true);
    assert!(loader.load());
    let _ = api_hash(sys::CEF_API_VERSION_LAST, 0);
    let args = Args::new();
    std::process::exit(execute_process(Some(args.as_main_args()), None::<&mut App>, std::ptr::null_mut()));
}

#[cfg(not(target_os = "macos"))]
fn main() {}
