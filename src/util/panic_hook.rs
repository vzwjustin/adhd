/// Install a panic hook that prints a useful message.
pub fn install() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // Ensure panic output is visible
        eprintln!("\n\x1b[31manchor panicked!\x1b[0m");
        original_hook(panic_info);
    }));
}
