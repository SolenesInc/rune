//! A binary with a dormant `procwatch::alloc::Counting`: it counts nothing until started.

#[global_allocator]
static ALLOCATOR: procwatch::alloc::Counting =
    procwatch::alloc::Counting::dormant(std::alloc::System);

#[test]
fn a_dormant_allocator_counts_nothing_until_started() {
    drop(std::hint::black_box(vec![0u8; 4096]));
    assert_eq!(procwatch::alloc::thread_allocated_bytes(), None);
    assert_eq!(procwatch::alloc::heap(), None);

    ALLOCATOR.start();
    drop(std::hint::black_box(vec![0u8; 4096]));
    assert_eq!(procwatch::alloc::thread_allocated_bytes(), Some(4096));
    assert_eq!(procwatch::alloc::thread_allocations(), 1);
}
