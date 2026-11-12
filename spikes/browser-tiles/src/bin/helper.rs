//! The CEF helper executable: every Chromium subprocess (renderer, GPU, network, storage) runs this.
use cef::{args::Args, *};

fn main() {
    let loader = library_loader::LibraryLoader::new(&std::env::current_exe().unwrap(), true);
    assert!(loader.load());
    let _ = api_hash(sys::CEF_API_VERSION_LAST, 0);
    let args = Args::new();
    std::process::exit(execute_process(Some(args.as_main_args()), None::<&mut App>, std::ptr::null_mut()));
}
