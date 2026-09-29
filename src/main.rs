mod llm;

use std::{
    env,
    fs::File,
    io::{self, BufRead, BufReader, IsTerminal, Read, Write},
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

    let mut request = env::args().skip(1).collect::<Vec<_>>().join(" ");
    if request.trim().is_empty() {
        // No args: read the request from stdin, so text with quotes/backticks
        // never has to pass through the shell's parser.
        if io::stdin().is_terminal() {
            eprintln!("Paste or type your request. When done, press Enter then Ctrl-D to submit:");
        }
        io::stdin().read_to_string(&mut request)?;
    }
    if request.trim().is_empty() {
        bail!("usage: spk <describe the command you want>  (or run `spk` and paste)");
    }

    let command = llm::generate_command(&request)?;
    println!("{command}");

    print!("Run this command? [y/N] ");
    io::stdout().flush()?;
    // Ask via the terminal directly, since stdin may already be used up.
    let tty = File::open("/dev/tty").context("no terminal available to confirm")?;
    let mut answer = String::new();
    BufReader::new(tty.try_clone()?).read_line(&mut answer)?;
    if !matches!(answer.trim().to_lowercase().as_str(), "y" | "yes") {
        return Ok(());
    }

    let shell = env::var("SHELL").unwrap_or_else(|_| "sh".to_owned());
    let status = Command::new(shell)
        .arg("-c")
        .arg(&command)
        .stdin(tty)
        .status()
        .context("failed to run command")?;
    process::exit(status.code().unwrap_or(1));
}
