use std::sync::OnceLock;
use super::traits::AppHandler;

static mut APP: OnceLock<*mut dyn AppHandler> = OnceLock::new();

pub fn set_app_handler<A: AppHandler>(app: &mut A) {
    unsafe {
        // SAFETY: This is only accessed on the main thread once during initialization.
        #[allow(static_mut_refs)]
        let _ = APP.set(app as *mut A as *mut dyn AppHandler);
    }
}

pub fn with_app_handler<F, R>(f: F) -> Option<R>
where
    F: FnOnce(*mut dyn AppHandler) -> R,
{
    // SAFETY: main-thread-only access (from main/UI thread) to static mutable pointer
    unsafe {
        #[allow(static_mut_refs)]
        APP.get().map(|&ptr| {
            f(ptr)
        })
    }
}