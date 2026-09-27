mod llm;

use std::{
    env,
    io::{self, Write},
    process::{self, Command},
};

use anyhow::{Context, Result, bail};

fn main() {
    if let Err(err) = run() {
        eprintln!("spk: {err:#}");
        process::exit(1);
    }
}

fn run() -> Result<()> {
    // Load the repo's .env (path fixed at build time). Existing env vars take precedence.
    let _ = dotenvy::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/.env"));

    let request = env::args().skip(1).collect::<Vec<_>>().join(" ");
    if request.trim().is_empty() {
        bail!("usage: spk <describe the command you want>");
    }

    let command = llm::generate_command(&request)?;
    println!("{command}");

    print!("Run this command? [y/N] ");
    io::stdout().flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    if !matches!(answer.trim().to_lowercase().as_str(), "y" | "yes") {
        return Ok(());
    }

    let shell = env::var("SHELL").unwrap_or_else(|_| "sh".to_owned());
    let status = Command::new(shell)
        .arg("-c")
        .arg(&command)
        .status()
        .context("failed to run command")?;
    process::exit(status.code().unwrap_or(1));
}
