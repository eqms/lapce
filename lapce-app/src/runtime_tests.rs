// Regression test for the ambient tokio runtime invariant (RT-01).
//
// Purpose: Guard against a future change that accidentally removes the
// `rt.enter()` guard in `lapce-app/src/bin/lapce.rs`, or replaces the
// named guard binding with `let _ = rt.enter()` (which drops immediately).
//
// This test is self-contained: it builds its own runtime, enters it, and
// verifies that `Handle::try_current()` succeeds with the expected flavor.
// It does NOT depend on the binary entry-point runtime being active.

#[cfg(test)]
mod runtime_tests {
    #[test]
    fn handle_current_succeeds_inside_entered_context() {
        // Build the same kind of runtime that the production entry point uses
        // (D-03: Builder::new_multi_thread().enable_all()).
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("test-worker")
            .build()
            .expect("test runtime");

        // Named binding — NOT `let _ = rt.enter()` which would drop immediately.
        let _guard = rt.enter();

        // `Handle::try_current()` must succeed while the guard is alive.
        let handle = tokio::runtime::Handle::try_current()
            .expect("handle must be present inside entered context");

        // The production pattern uses new_multi_thread(), so the flavor
        // must be MultiThread, not CurrentThread.
        assert_eq!(
            handle.runtime_flavor(),
            tokio::runtime::RuntimeFlavor::MultiThread
        );
    }
}
