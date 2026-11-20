//! The thread that polls a renderer's device so its submissions' completions fire.

use std::sync::mpsc::{sync_channel, SyncSender, TrySendError};
use std::thread::JoinHandle;

/// Polls the device after each wakeup until its queue is empty, and is joined when dropped, so
/// no poll outlives the renderer that owns it. A detached poller could still be inside the
/// driver when the process exits and tears the driver down, which NVIDIA's Vulkan driver
/// answers with a segfault.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(super) struct Completion {
    wake: Option<SyncSender<()>>,
    thread: Option<JoinHandle<()>>,
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
impl Completion {
    /// `poll` polls the device once without blocking.
    pub(super) fn spawn(mut poll: impl FnMut() -> Result<wgpu::PollStatus, wgpu::PollError> + Send + 'static) -> Self {
        let (wake, receiver) = sync_channel(1);
        let thread = std::thread::Builder::new()
            .name("rune-video-completion".into())
            .spawn(move || {
                // Nonblocking poll releases wgpu's snatch lock before sleeping. A fence wait here
                // would keep that read lock and serialize queue.present's write lock behind GPU work.
                // Wakeups are coalesced; the final frame still completes after the window goes idle.
                while receiver.recv().is_ok() {
                    loop {
                        match poll() {
                            Ok(wgpu::PollStatus::QueueEmpty) => break,
                            Ok(_) => std::thread::sleep(std::time::Duration::from_millis(1)),
                            Err(_) => return,
                        }
                    }
                }
            })
            .expect("video completion thread");
        Self { wake: Some(wake), thread: Some(thread) }
    }

    /// A sender that wakes the thread, for a completion callback that submits more work.
    pub(super) fn waker(&self) -> SyncSender<()> {
        self.wake.clone().expect("a live completion thread")
    }

    /// Wakes the thread; false when a wakeup is already pending.
    pub(super) fn wake(&self) -> bool {
        !matches!(self.wake.as_ref().map(|w| w.try_send(())), Some(Err(TrySendError::Full(()))))
    }
}

impl Drop for Completion {
    /// The thread leaves once the queue is empty and every sender is gone. Callbacks holding a
    /// `waker` fire by then: they belong to work the thread polls to completion.
    fn drop(&mut self) {
        drop(self.wake.take());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
#[path = "completion_tests.rs"]
mod tests;
