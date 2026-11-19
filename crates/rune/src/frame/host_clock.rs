//! The host's monotonic clock in nanoseconds and the calling thread's CPU time, both precise.

/// Nanoseconds on the host's monotonic clock: `mach_absolute_time` through its timebase, else
/// `CLOCK_MONOTONIC`.
#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(not(feature = "frame-profile"), allow(dead_code))]
pub(crate) fn now_ns() -> u64 {
    #[cfg(target_vendor = "apple")]
    {
        #[repr(C)]
        struct Timebase {
            numer: u32,
            denom: u32,
        }
        unsafe extern "C" {
            fn mach_timebase_info(info: *mut Timebase) -> i32;
            fn mach_absolute_time() -> u64;
        }
        static BASE: std::sync::OnceLock<(u32, u32)> = std::sync::OnceLock::new();
        let (numer, denom) = *BASE.get_or_init(|| {
            let mut info = Timebase { numer: 0, denom: 0 };
            // SAFETY: `info` is a live, writable timebase struct.
            unsafe { mach_timebase_info(&mut info) };
            (info.numer.max(1), info.denom.max(1))
        });
        // SAFETY: no preconditions.
        let ticks = unsafe { mach_absolute_time() };
        (ticks as u128 * numer as u128 / denom as u128) as u64
    }
    #[cfg(all(unix, not(target_vendor = "apple")))]
    {
        let mut t = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        // SAFETY: `t` is a live, writable timespec.
        unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut t) };
        t.tv_sec as u64 * 1_000_000_000 + t.tv_nsec as u64
    }
}

/// The calling thread's CPU time in nanoseconds.
#[cfg(unix)]
pub(crate) fn thread_cpu() -> u64 {
    let mut t = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `t` is a live, writable timespec and the clock needs no handle.
    let ok = unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut t) };
    assert_eq!(ok, 0, "thread CPU clock unavailable");
    t.tv_sec as u64 * 1_000_000_000 + t.tv_nsec as u64
}
