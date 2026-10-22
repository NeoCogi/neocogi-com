//! Owns the stack budget for recursive Rust parsing, printing and source mapping.
//!
//! Windows build-script main threads have a smaller default stack than Unix.
//! Compiler-owned Rust text and stable source coordinates cross this boundary;
//! thread-local proc-macro spans and syntax trees stay inside their owning worker.

use super::GenerateError;

/// One platform-independent budget for the generated-Rust syntax phase.
/// Deep valid output-buffer types must not depend on the caller's thread stack.
const SYNTAX_STACK_BYTES: usize = 8 * 1024 * 1024;

/// Runs one syntax operation with borrowed, typed inputs and returns its result.
/// Spawn failure is a generation error; an internal panic retains its original
/// payload instead of being reclassified as an authored IDL diagnostic.
pub(super) fn with_syntax_stack<T: Send>(work: impl FnOnce() -> Result<T, GenerateError> + Send) -> Result<T, GenerateError> {
    std::thread::scope(|scope| {
        let worker = std::thread::Builder::new()
            .name("neocogi-idl-syntax".into())
            .stack_size(SYNTAX_STACK_BYTES)
            .spawn_scoped(scope, work)
            .map_err(|error| GenerateError::new(format!("cannot start generated-Rust syntax worker: {error}")))?;
        match worker.join() {
            Ok(result) => result,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    })
}
