//! Owns the stack budget for recursive IDL/Rust parsing, printing and mapping.
//!
//! Windows build-script main threads have a smaller default stack than Unix.
//! Compiler-owned text and stable source models cross this boundary;
//! thread-local proc-macro spans and syntax trees stay inside their owning worker.

/// One platform-independent budget for the compiler's recursive syntax phases.
/// Deep valid output-buffer types must not depend on the caller's thread stack.
const SYNTAX_STACK_BYTES: usize = 8 * 1024 * 1024;

#[cfg(test)]
mod tests;

/// Runs one syntax operation with borrowed, typed inputs and returns its result.
/// Spawn failure stays an I/O error for the owning phase to classify. An internal
/// panic retains its payload instead of becoming an authored IDL diagnostic.
pub(crate) fn with_syntax_stack<T: Send>(work: impl FnOnce() -> T + Send) -> std::io::Result<T> {
    #[cfg(test)]
    failure::before_start()?;
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

/// Caller-thread failure selection for unit tests; no policy is linked into consumers.
#[cfg(test)]
pub(crate) mod failure {
    use std::{cell::Cell, io, marker::PhantomData, rc::Rc};

    /// One-shot denial after a known number of allowed worker starts.
    #[derive(Clone, Copy)]
    enum Selection {
        /// Admit ordinary syntax work.
        Allowed,
        /// Deny once the remaining permitted starts have been attempted.
        After(usize),
    }

    std::thread_local! {
        /// Private to the caller: workers and unrelated test threads inherit no denial.
        static SELECTED: Cell<Selection> = const { Cell::new(Selection::Allowed) };
    }

    /// Restores the enclosing selection on this same thread, including on unwind.
    /// Rc's marker prevents moving the restoration obligation to another thread.
    pub(crate) struct Denial {
        /// Enclosing selection restored rather than globally reset.
        previous: Selection,
        /// Binds Drop to the thread whose selection was changed.
        _thread: PhantomData<Rc<()>>,
    }

    impl Drop for Denial {
        /// Restores only this caller's enclosing test selection.
        fn drop(&mut self) {
            SELECTED.set(self.previous);
        }
    }

    /// Denies one selected start; callers must assert its phase-specific error.
    pub(crate) fn after(allowed: usize) -> Denial {
        Denial {
            previous: SELECTED.replace(Selection::After(allowed)),
            _thread: PhantomData,
        }
    }

    /// Consumes the denial before returning an error, so a later start can succeed.
    pub(super) fn before_start() -> io::Result<()> {
        match SELECTED.get() {
            Selection::Allowed => Ok(()),
            Selection::After(0) => {
                SELECTED.set(Selection::Allowed);
                Err(io::Error::new(io::ErrorKind::WouldBlock, "test denied syntax worker start"))
            }
            Selection::After(remaining) => {
                SELECTED.set(Selection::After(remaining - 1));
                Ok(())
            }
        }
    }
}
