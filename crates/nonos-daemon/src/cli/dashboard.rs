use nonos_types::NonosResult;
use std::path::PathBuf;

pub async fn launch_dashboard(data_dir: &PathBuf, theme: &str) -> NonosResult<()> {
    println!("\x1b[38;5;46mLaunching NONOS Dashboard...\x1b[0m");
    println!();
    println!("Theme: \x1b[38;5;51m{}\x1b[0m", theme);
    println!("Data:  \x1b[38;5;51m{:?}\x1b[0m", data_dir);
    println!();
    println!("\x1b[38;5;226mNote:\x1b[0m The TUI dashboard is provided by the separate \x1b[38;5;51mnonos-dash\x1b[0m binary.");
    println!();
    println!("If installed, you can run it directly:");
    println!("  \x1b[38;5;51mnonos-dash --theme {}\x1b[0m", theme);
    println!();
    println!("Or install it with:");
    println!("  \x1b[38;5;51mcargo install --path crates/nonos-dash\x1b[0m");

    #[cfg(unix)]
    {
        use std::process::Command;
        if let Ok(status) = Command::new("nonos-dash")
            .arg("--theme")
            .arg(theme)
            .status()
        {
            if !status.success() {
            }
        }
    }

    Ok(())
}
