use super::*;
use std::sync::Arc;

/// A renderer's poller still inside a poll when its renderer is gone can still be inside the
/// driver when the process exits, which crashed the lib test process on NVIDIA after every
/// test passed. The poll here reports work for about 100 ms, so a poller left running holds
/// it well past the drop; a joined one has let it go by the time the drop returns.
#[test]
fn dropping_the_completion_thread_waits_for_its_poll_to_finish() {
    let held = Arc::new(());
    let in_poll = Arc::downgrade(&held);
    let (entered, polling) = std::sync::mpsc::channel();
    let mut polls = 0;
    let completion = Completion::spawn(move || {
        let _device = &held;
        polls += 1;
        if polls == 1 { let _ = entered.send(()); }
        Ok(if polls < 100 { wgpu::PollStatus::Poll } else { wgpu::PollStatus::QueueEmpty })
    });
    assert!(completion.wake());
    polling.recv().expect("the thread polls after a wakeup");
    drop(completion);
    assert!(in_poll.upgrade().is_none(), "the completion thread outlived its owner, still polling");
}
