use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};

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
        #[clap(flatten)]
        random: RandomOptions,
        /// Load native BASIC source and pause in the monitor before RUN.
        #[clap(long, requires = "native")]
        break_before_run: bool,
        /// Run the native R8 ROM interpreter.
        #[clap(long)]
        native: bool,
        /// Select the reference interpreter for a live session.
        #[clap(long, requires = "live", conflicts_with = "native")]
        reference: bool,
        /// Select video keyboard or terminal console for a live session.
        #[clap(long, requires = "live", value_enum, default_value = "screen")]
        console: ConsoleOption,
        /// Open the live video window and speaker (requires the live Cargo feature).
        #[clap(long)]
        live: bool,
        /// Run live emulation without real-time pacing; live audio is silent.
        #[clap(long, requires = "live")]
        turbo: bool,
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
        #[clap(flatten)]
        random: RandomOptions,
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
        #[clap(flatten)]
        random: RandomOptions,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum ConsoleOption {
    Screen,
    Serial,
}

#[derive(Clone, Debug, Subcommand)]
enum DocCommand {
    /// Generate opcode table.
    Opcodes,
}

/// Optional hardware shared by BASIC, the monitor, and assembly programs.
#[derive(Debug, clap::Args)]
struct RandomOptions {
    /// Plug in the random device at FF20..FF23.
    #[clap(long)]
    random_device: bool,
    /// Initial 32-bit seed for repeatable random sequences (decimal).
    #[clap(long, requires = "random_device")]
    random_seed: Option<u32>,
}

impl RandomOptions {
    fn device(&self) -> Result<Option<rx82::random::RandomDevice>> {
        if !self.random_device {
            return Ok(None);
        }
        Ok(Some(if let Some(seed) = self.random_seed {
            rx82::random::RandomDevice::with_seed(seed)
        } else {
            rx82::random::RandomDevice::from_entropy()?
        }))
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Basic {
            break_before_run,
            native,
            reference,
            console,
            live,
            turbo,
            path,
            step,
            random,
        } => {
            let random = random.device()?;
            if live {
                #[cfg(feature = "live")]
                {
                    anyhow::ensure!(
                        !break_before_run && !step,
                        "live mode does not combine with debugger options"
                    );
                    let source = path.map(fs::read_to_string).transpose()?;
                    return if reference {
                        let route = if console == ConsoleOption::Screen {
                            rx82::native::ConsoleRoute::Screen
                        } else {
                            rx82::native::ConsoleRoute::Serial
                        };
                        rx82::live::run_reference(source, turbo, random, route)
                    } else {
                        let route = if console == ConsoleOption::Screen {
                            rx82::native::ConsoleRoute::Screen
                        } else {
                            rx82::native::ConsoleRoute::Serial
                        };
                        rx82::live::run_native(source, turbo, random, route)
                    };
                }
                #[cfg(not(feature = "live"))]
                anyhow::bail!("live mode requires building with --features live");
            }
            let _ = (turbo, reference, console);
            if native {
                let source = path.map(fs::read_to_string).transpose()?;
                if step || break_before_run {
                    let mut monitor = rx82::native::debug_monitor(
                        source.as_deref().ok_or_else(|| {
                            anyhow::anyhow!("native debugging requires a BASIC source file")
                        })?,
                        break_before_run,
                    )?;
                    if let Some(device) = random {
                        monitor.sys.devices.insert(0, Box::new(device));
                    }
                    if break_before_run {
                        println!("BASIC source loaded; paused before RUN. Use M 1000, then G.");
                    }
                    return monitor.interact_at_current_pc();
                }
                let mut devices: Vec<Box<dyn rx82::system::Device>> = Vec::new();
                if let Some(device) = random {
                    devices.push(Box::new(device));
                }
                return rx82::native::interact_with_devices(
                    source.as_deref(),
                    &mut std::io::stdin().lock(),
                    &mut std::io::stdout().lock(),
                    devices,
                );
            }
            let mut basic = rx82::basic::Basic::default();
            if let Some(device) = random {
                basic.attach_random(device);
            }
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
            random,
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
            if let Some(device) = random.device()? {
                mon.sys.devices.insert(0, Box::new(device));
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
            random,
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
                if let Some(device) = random.device()? {
                    mon.sys.devices.insert(0, Box::new(device));
                }
                mon.step = step;
                mon.sys.turbo = turbo;
                mon.run_program(&program)?;
            }
            Ok(())
        }
    }
}
