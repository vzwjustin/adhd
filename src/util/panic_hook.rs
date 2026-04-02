use crossterm::{
    execute,
    terminal::{disable_raw_mode, LeaveAlternateScreen},
};
use std::io::stderr;

/// Install a panic hook that restores the terminal before printing the panic.
/// This is critical — without it, a panic leaves the terminal in raw mode.
pub fn install() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // Best-effort terminal restore
        let _ = disable_raw_mode();
        let _ = execute!(stderr(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));
}
