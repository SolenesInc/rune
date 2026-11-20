use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

pub(super) static BUILDS: AtomicUsize = AtomicUsize::new(0);

#[test]
fn threads_share_one_system_collection() {
    let ask = || std::thread::spawn(|| family_files(&["DejaVu Sans"])).join().unwrap();
    assert_eq!(ask(), ask());
    assert_eq!(BUILDS.load(Ordering::Relaxed), 1);
}
