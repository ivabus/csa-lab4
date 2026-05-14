mod disasm;
mod tester;

use color_eyre::eyre::{ContextCompat, Result};
use emulator::{IO, Processor, cpu};

use std::{collections::BTreeMap, path::PathBuf};

use clap::Parser;

#[derive(clap::Parser)]
enum Commands {
    /// Build source into source.bin
    #[command(alias = "b")]
    Build {
        source_file: PathBuf,
        /// Emit textual assembly listing (source.asm)
        #[arg(long = "emit-asm")]
        emit_asm: bool,
    },
    /// Build and execute file in VM
    #[command(alias = "r")]
    Run {
        source_file: PathBuf,
        /// Trace depth: -v for commands, -vv for microcommands
        #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count)]
        trace_level: u8,
    },
    /// Test using custom test framework
    #[command()]
    Test { test_file: PathBuf },
}

fn print_trace(s: &str) {
    print!("{}", s);
}

fn run(binary: &[u8], trace_level: u8, io: (impl std::io::Read, impl std::io::Write)) {
    if trace_level > 0 {
        *cpu::TRACE_TARGET.write().unwrap() = print_trace;
    }
    let mut memory = BTreeMap::new();
    for (i, chunk) in binary.chunks(4).enumerate() {
        if chunk.len() == 4 {
            let val = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            memory.insert(0x1000 + (i as u32 * 4), val);
        }
    }

    let mut io_map = BTreeMap::new();
    io_map.insert(0x0000_0000, IO::I(Box::new(io.0)));
    io_map.insert(0x0000_0004, IO::O(Box::new(io.1)));

    let mut cpu = Processor::new(memory, io_map);
    cpu.trace = trace_level;

    let mut limit = 100_000_000;
    while cpu.step() && limit > 0 {
        limit -= 1;
    }
}

// Green PRINT
#[macro_export]
macro_rules! grint {
	($msg:tt, $($arg:tt)*) => {{
		use colored::Colorize;
        ::std::println!("{:>12} {}", $msg.green().bold(), ::std::format_args!($($arg)*));
    }}
}

// ERror PRINT
#[macro_export]
macro_rules! errint {
	($msg:tt, $($arg:tt)*) => {{
		use colored::Colorize;
        ::std::println!("{:>12} {}", $msg.red().bold(), ::std::format_args!($($arg)*));
    }}
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let command = Commands::parse();
    match command {
        Commands::Build {
            mut source_file,
            emit_asm,
        } => {
            grint!(
                "Building",
                "`{}`",
                source_file
                    .canonicalize()?
                    .to_str()
                    .context("Cannot translate path to string")?
            );
            let translated = translator::translate(&source_file)?;

            if emit_asm {
                let mut asm_path = source_file.clone();
                asm_path.set_extension("asm");
                let listing = disasm::disasm(&translated);
                std::fs::write(&asm_path, &listing)?;
                grint!(
                    "Written",
                    "`{}`",
                    asm_path
                        .canonicalize()?
                        .to_str()
                        .context("Cannot translate path to string")?
                );
            }

            source_file.set_extension("bin");
            std::fs::write(&source_file, translated)?;
            grint!(
                "Written",
                "`{}`",
                source_file
                    .canonicalize()?
                    .to_str()
                    .context("Cannot translate path to string")?
            );
        }
        Commands::Run {
            source_file,
            trace_level,
        } => {
            grint!(
                "Building",
                "`{}`",
                source_file
                    .canonicalize()?
                    .to_str()
                    .context("Cannot translate path to string")?
            );
            let binary = translator::translate(&source_file)?;
            grint!(
                "Running",
                "`{}`",
                source_file
                    .canonicalize()?
                    .to_str()
                    .context("Cannot translate path to string")?
            );
            run(&binary, trace_level, (std::io::stdin(), std::io::stdout()));
        }
        Commands::Test { test_file } => tester::test(test_file)?,
    }
    Ok(())
}
