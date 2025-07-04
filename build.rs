use clap::{CommandFactory, Parser};
use clap_complete::aot::Fish;
use clap_complete::{generate_to, shells};
use std::env;
use std::io::Error;

include!("src/cli.rs");

fn main() -> Result<(), Error> {
    let mut cmd = Cli::command();
    let path = generate_to(
        shells::Fish,
        &mut cmd, // We need to specify what generator to use
        "gg",     // We need to specify the bin name manually
        ".",      // We need to specify where to write to
    )?;

    println!("cargo:warning=completion file is generated: {path:?}");

    Ok(())
}
