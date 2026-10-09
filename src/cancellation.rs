//! CLI-owned cooperative Ctrl-C cancellation. Signal callbacks only set a flag;
//! normal request code performs cleanup and emits the final response.
use crate::domain::{Error, Result};
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};

static REQUESTED: AtomicBool = AtomicBool::new(false);
thread_local! {
    static FINALIZING: Cell<usize> = const { Cell::new(0) };
}

pub fn check() -> Result<()> {
    if REQUESTED.load(Ordering::Relaxed) && !FINALIZING.with(|depth| depth.get() > 0) {
        Err(Error::new("CANCELLED", "Request cancelled by user", 130))
    } else {
        Ok(())
    }
}

/// Finish already-observed execution truth, owned resource cleanup and response
/// publication. This suppresses only cancellation, never the original deadline.
#[must_use]
pub struct FinalizationGuard {
    // A guard must remain on the thread whose cancellation checks it suppresses.
    _thread: std::marker::PhantomData<std::rc::Rc<()>>,
}
pub fn finalize() -> FinalizationGuard {
    FINALIZING.with(|depth| depth.set(depth.get() + 1));
    FinalizationGuard {
        _thread: std::marker::PhantomData,
    }
}
impl Drop for FinalizationGuard {
    fn drop(&mut self) {
        FINALIZING.with(|depth| depth.set(depth.get() - 1));
    }
}

/// Install only in the executable, never implicitly for library clients or
/// private guardian processes. SIGTERM and child-native signals keep their own
/// contracts; this handler represents an explicit user Ctrl-C.
pub fn install() -> Result<()> {
    #[cfg(unix)]
    {
        extern "C" fn interrupt(_: libc::c_int) {
            REQUESTED.store(true, Ordering::Relaxed);
        }
        // SAFETY: a fully initialized sigaction installs a static signal-safe
        // callback. The callback neither allocates nor touches request state.
        unsafe {
            let mut action: libc::sigaction = std::mem::zeroed();
            action.sa_sigaction = interrupt as *const () as usize;
            action.sa_flags = libc::SA_RESTART;
            libc::sigemptyset(&mut action.sa_mask);
            if libc::sigaction(libc::SIGINT, &action, std::ptr::null_mut()) != 0 {
                return Err(std::io::Error::last_os_error().into());
            }
        }
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Console::{CTRL_C_EVENT, SetConsoleCtrlHandler};
        unsafe extern "system" fn interrupt(event: u32) -> windows_sys::core::BOOL {
            if event == CTRL_C_EVENT {
                REQUESTED.store(true, Ordering::Relaxed);
                1
            } else {
                0
            }
        }
        // SAFETY: the static callback is valid for the process lifetime.
        if unsafe { SetConsoleCtrlHandler(Some(interrupt), 1) } == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
    }
    Ok(())
}
