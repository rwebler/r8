use anyhow::Result;
use clap::{Parser, Subcommand};

use std::{fs, path::PathBuf};

use r8asm::{Disassembler, assemble_source_file};
use rx82::{doc::opcodes, monitor::Monitor};

use crate::DocCommand::Opcodes;

/// An emulator for the RX82 fantasy retro computer system.
#[derive(Debug, Parser)]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Assemble a source file.
    Asm {
        /// Paths to the source files.
        paths: Vec<PathBuf>,
    },
    /// Start BASIC, or run a numbered BASIC source file.
    Basic {
        /// Run the native R8 ROM interpreter.
        #[clap(long)]
        native: bool,
        /// Single-step native execution in the monitor (requires a source file).
        #[clap(long, requires = "native")]
        step: bool,
        /// BASIC source file to run.
        path: Option<PathBuf>,
    },
    /// Disassemble a binary file.
    Dis {
        /// Paths to the binary files.
        paths: Vec<PathBuf>,
    },
    /// Generate documentation.
    Doc {
        #[clap(subcommand)]
        doc_cmd: DocCommand,
    },
    /// Start the interactive monitor.
    Mon {
        /// Skip running boot ROM.
        #[clap(long)]
        skiprom: bool,
        /// Single-step if loading a binary file.
        #[clap(short, long)]
        step: bool,
        /// Paths to binary files to run.
        paths: Option<Vec<PathBuf>>,
        /// Full host-native speed.
        #[clap(short, long)]
        turbo: bool,
    },
    /// Assemble and run a program in the monitor.
    Run {
        /// Skip running boot ROM.
        #[clap(long)]
        skiprom: bool,
        /// Single-step the program.
        #[clap(short, long)]
        step: bool,
        /// Paths to the source files.
        paths: Vec<PathBuf>,
        /// Full host-native speed.
        #[clap(short, long)]
        turbo: bool,
    },
}

#[derive(Clone, Debug, Subcommand)]
enum DocCommand {
    /// Generate opcode table.
    Opcodes,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Basic { native, path, step } => {
            if native {
                let source = path.map(fs::read_to_string).transpose()?;
                if step {
                    return rx82::native::debug(
                        source.as_deref().ok_or_else(|| {
                            anyhow::anyhow!("--step requires a BASIC source file")
                        })?,
                    );
                }
                return rx82::native::interact(
                    source.as_deref(),
                    &mut std::io::stdin().lock(),
                    &mut std::io::stdout().lock(),
                );
            }
            let mut basic = rx82::basic::Basic::default();
            let mut input = std::io::stdin().lock();
            let mut output = std::io::stdout().lock();
            if let Some(path) = path {
                basic.load(&fs::read_to_string(path)?)?;
                basic.run(&mut input, &mut output)
            } else {
                basic.interact(&mut input, &mut output)
            }
        }
        Command::Asm { paths } => {
            for path in paths {
                let data = assemble_source_file(&path)?;
                let mut bin_path = path.clone();
                bin_path.set_extension("bin");
                fs::write(bin_path, data)?;
            }
            Ok(())
        }
        Command::Dis { paths } => {
            for path in paths {
                let code = fs::read(&path)?;
                for source in Disassembler::from(code.as_slice()) {
                    println!("    {source}");
                }
            }
            Ok(())
        }
        Command::Doc { doc_cmd } => {
            match doc_cmd {
                Opcodes => opcodes(),
            }
            Ok(())
        }
        Command::Mon {
            paths: maybe_paths,
            skiprom,
            step,
            turbo,
        } => {
            let mut programs = Vec::new();
            if let Some(paths) = maybe_paths {
                for path in paths {
                    let program = fs::read(path)?;
                    programs.push(program);
                }
            }
            let mut mon = Monitor::default();
            if !skiprom {
                mon.sys.reset();
            }
            mon.step = step;
            mon.sys.turbo = turbo;
            if programs.is_empty() {
                mon.interact()?;
            } else {
                for program in programs {
                    mon.run_program(&program)?;
                }
            }
            Ok(())
        }
        Command::Run {
            paths,
            skiprom,
            step,
            turbo,
        } => {
            for path in paths {
                let program = assemble_source_file(&path)?;
                let mut mon = Monitor::default();
                if !skiprom {
                    mon.sys.reset();
                }
                mon.step = step;
                mon.sys.turbo = turbo;
                mon.run_program(&program)?;
            }
            Ok(())
        }
    }
}
