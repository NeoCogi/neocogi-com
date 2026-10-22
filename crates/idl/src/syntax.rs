//! Owns the stack budget for recursive IDL/Rust parsing, printing and mapping.
//!
//! Windows build-script main threads have a smaller default stack than Unix.
//! Compiler-owned text and stable source models cross this boundary;
//! thread-local proc-macro spans and syntax trees stay inside their owning worker.

/// One platform-independent budget for the compiler's recursive syntax phases.
/// Deep valid output-buffer types must not depend on the caller's thread stack.
const SYNTAX_STACK_BYTES: usize = 8 * 1024 * 1024;

/// Runs one syntax operation with borrowed, typed inputs and returns its result.
/// Spawn failure stays an I/O error for the owning phase to classify. An internal
/// panic retains its payload instead of becoming an authored IDL diagnostic.
pub(crate) fn with_syntax_stack<T: Send>(work: impl FnOnce() -> T + Send) -> std::io::Result<T> {
    std::thread::scope(|scope| {
        let worker = std::thread::Builder::new()
            .name("neocogi-idl-syntax".into())
            .stack_size(SYNTAX_STACK_BYTES)
            .spawn_scoped(scope, work)?;
        match worker.join() {
            Ok(result) => Ok(result),
            Err(panic) => std::panic::resume_unwind(panic),
        }
    })
}
