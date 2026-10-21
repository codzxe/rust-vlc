//! End-to-end proof of the counting-allocation instrument itself
//! (ED-FIX-2): this binary installs
//! [`CountingGlobalAlloc`](testkit::alloc_count::CountingGlobalAlloc)
//! as its **process** allocator and proves the properties the
//! zero-allocation test suites rely on:
//!
//! 1. a heap allocation inside a guard window is *seen* (bytes and
//!    events), and a drop is seen;
//! 2. a silent window asserts zero cleanly;
//! 3. an allocation inside a window makes `assert_zero` fail loudly;
//! 4. snapshots expose the running totals.
//!
//! # Why exactly one `#[test]`
//!
//! The global allocator is process-global and counting is
//! unconditional, so *any* thread allocating inside a measured window
//! is seen — including the harness's own reporting (a sibling test's
//! completion line being printed by the main thread allocates). This
//! binary therefore runs **one** test that executes the scenarios
//! sequentially; there are no sibling threads and no concurrent
//! reporting, which keeps the windows deterministic under the default
//! parallel `cargo test` invocation (single-threaded *behavior* by
//! construction, not by a flag the gate would have to remember).
//!
//! This is the D-6 discipline applied to the instrument before any
//! card is allowed to cite it: the measurement tool itself is
//! measured and proven, not trusted.

#[global_allocator]
static ALLOC: testkit::alloc_count::CountingGlobalAlloc =
    testkit::alloc_count::CountingGlobalAlloc::new();

use testkit::alloc_count::serialize_tests;

#[test]
fn instrument_end_to_end() {
    let _serial = serialize_tests();

    // 1. A real heap allocation and its drop are seen.
    let guard = ALLOC.state().guard();
    let bump = vec![0u8; 4096];
    let delta = guard.delta();
    assert!(
        delta.alloc_calls >= 1,
        "a Vec allocation must be counted: {delta:?}"
    );
    assert!(
        delta.total_bytes >= 4096,
        "the Vec's bytes must be counted: {delta:?}"
    );
    drop(bump);
    let delta = guard.delta();
    assert_eq!(
        delta.live_bytes, 0,
        "after the drop, live bytes return to the window's start"
    );
    assert!(delta.dealloc_calls >= 1);

    // 2. A silent window asserts zero cleanly (pure stack/register
    //    work between the snapshots).
    let guard = ALLOC.state().guard();
    let mut acc = 0u64;
    for i in 0..10_000u64 {
        acc = acc.wrapping_add(i * i);
    }
    assert!(acc > 0, "the compiler may not elide the loop body");
    guard.assert_zero("pure arithmetic window");

    // 3. Snapshots expose the running totals.
    let before = ALLOC.state().snapshot();
    let bump = vec![1u8; 256];
    let after = ALLOC.state().snapshot();
    drop(bump);
    assert!(after.total_bytes >= before.total_bytes + 256);
    assert!(after.alloc_calls >= before.alloc_calls + 1);

    // 4. An allocation inside a window fails loudly (proven via
    //    catch_unwind so this binary stays green while the failure
    //    path is exercised end to end; the panic's own formatting
    //    allocations happen inside the deliberately-dirty window).
    let caught = std::panic::catch_unwind(|| {
        let guard = ALLOC.state().guard();
        let _bump = vec![7u8; 128];
        guard.assert_zero("this window is dirty by construction");
    });
    assert!(caught.is_err(), "assert_zero must fail on a dirty window");

    // 5. After all that noise, a fresh window is clean again — the
    //    instrument has no stickiness.
    let guard = ALLOC.state().guard();
    let sum: u64 = (0..100u64).sum();
    assert_eq!(sum, 4_950);
    guard.assert_zero("post-noise window");
}
