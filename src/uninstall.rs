//! Handles `snag uninstall` — removes the binary and optionally config/data.

use crate::config;
use anyhow::Result;
use std::io::{self, Write};

pub fn run() -> Result<()> {
    println!();
    println!("This will remove snag from your system.");
    println!();

    let exe = std::env::current_exe()?;
    println!("Binary:  {}", exe.display());
    println!("Config:  {}", config::config_dir().display());
    println!("Data:    {}", config::data_dir().display());
    println!();

    if !confirm("Remove snag binary?")? {
        println!("Aborted.");
        return Ok(());
    }

    let remove_data = confirm("Also remove config, alerts, results, and credentials?")?;

    println!();

    if remove_data {
        let config_dir = config::config_dir();
        if config_dir.exists() {
            std::fs::remove_dir_all(&config_dir)?;
            println!("Removed config: {}", config_dir.display());
        }

        let data_dir = config::data_dir();
        if data_dir.exists() {
            std::fs::remove_dir_all(&data_dir)?;
            println!("Removed data:   {}", data_dir.display());
        }
    }

    // Remove the binary last (since we're running from it)
    // On Unix, unlinking while running is fine — the OS keeps the file until the process exits.
    if exe.exists() {
        std::fs::remove_file(&exe)?;
        println!("Removed binary: {}", exe.display());
    }

    println!();
    println!("snag has been uninstalled.");

    Ok(())
}

fn confirm(prompt: &str) -> Result<bool> {
    print!("{} [y/N] ", prompt);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().eq_ignore_ascii_case("y"))
}
