//! Global interrupt control.

unsafe extern "C" {
    #[link_name = "enable_interrupts"]
    fn enable_interrupts_asm();

    fn save_and_disable_interrupts() -> u8;
    fn restore_interrupts(status: u8);
}

/// Enables global CPU interrupts.
///
/// # Safety
///
/// Interrupt handlers and shared state must already be initialized.
pub unsafe fn enable_global() {
    unsafe {
        enable_interrupts_asm();
    }
}

/// Runs operation with interrupts disabled, then restores the previous state
pub fn with_disabled<R>(operation: impl FnOnce() -> R) -> R {
    let status = unsafe { save_and_disable_interrupts() };
    let result = operation();
    unsafe { restore_interrupts(status) };
    result
}
