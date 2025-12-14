use clap::{Parser, Subcommand};
use hyprsnip_config::Config;
use hyprsnip_utils::{Aggressiveness, TrimOptions};
use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Debug, Parser)]
#[command(name = "hyprsnip")]
#[command(about = "Wayland clipboard command trimmer", long_about = None)]
struct Cli {
    #[arg(long)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// One-shot trim (reads stdin, prints to stdout)
    Trim(TrimArgs),

    /// Config helpers
    Config {
        #[command(subcommand)]
        cmd: ConfigCmd,
    },

    /// Clipboard daemon (stub)
    Daemon(DaemonArgs),

    /// systemd user service helpers (stub)
    Service {
        #[command(subcommand)]
        cmd: ServiceCmd,
    },
}

#[derive(Debug, Parser)]
struct TrimArgs {
    #[arg(long)]
    aggressiveness: Option<AggressivenessArg>,

    #[arg(long, conflicts_with = "no_keep_blank_lines")]
    keep_blank_lines: bool,

    #[arg(long, conflicts_with = "keep_blank_lines")]
    no_keep_blank_lines: bool,

    #[arg(long, conflicts_with = "no_remove_box_drawing")]
    remove_box_drawing: bool,

    #[arg(long, conflicts_with = "remove_box_drawing")]
    no_remove_box_drawing: bool,

    /// Safety valve (applies here too)
    #[arg(long)]
    max_auto_lines: Option<usize>,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum AggressivenessArg {
    Low,
    Normal,
    High,
}

impl From<AggressivenessArg> for Aggressiveness {
    fn from(value: AggressivenessArg) -> Self {
        match value {
            AggressivenessArg::Low => Aggressiveness::Low,
            AggressivenessArg::Normal => Aggressiveness::Normal,
            AggressivenessArg::High => Aggressiveness::High,
        }
    }
}

#[derive(Debug, Subcommand)]
enum ConfigCmd {
    /// Print default config file path
    Path,

    /// Print the loaded effective config as TOML
    Print,

    /// Print a default config template as TOML
    Example,
}

#[derive(Debug, Parser)]
struct DaemonArgs {
    #[arg(long)]
    dry_run: bool,
}

#[derive(Debug, Subcommand)]
enum ServiceCmd {
    /// Print a systemd user unit file
    Unit,

    /// Print install instructions
    Install,

    /// Print uninstall instructions
    Uninstall,
}

fn main() -> ExitCode {
    if let Err(err) = run() {
        eprintln!("{err}");
        return ExitCode::from(1);
    }

    ExitCode::SUCCESS
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let config = Config::load(cli.config.as_deref())?;

    match cli.command {
        Command::Trim(args) => {
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input)?;

            let options = effective_trim_options(config.trim, args);
            let res = hyprsnip_utils::trim_text(&input, &options);
            print!("{}", res.trimmed);
            Ok(())
        }
        Command::Config { cmd } => match cmd {
            ConfigCmd::Path => {
                println!("{}", hyprsnip_config::default_config_path()?.display());
                Ok(())
            }
            ConfigCmd::Print => {
                println!("{}", config.to_toml_pretty()?);
                Ok(())
            }
            ConfigCmd::Example => {
                println!("{}", Config::default_toml()?);
                Ok(())
            }
        },
        Command::Daemon(args) => {
            if args.dry_run {
                println!("daemon not implemented (dry-run)");
            } else {
                println!("daemon not implemented");
            }
            Ok(())
        }
        Command::Service { cmd } => {
            match cmd {
                ServiceCmd::Unit => {
                    print!("{}", systemd_user_unit());
                }
                ServiceCmd::Install => {
                    println!("Write this to ~/.config/systemd/user/hyprsnip.service:\n");
                    println!("{}", systemd_user_unit());
                    println!("Then run:\n  systemctl --user daemon-reload\n  systemctl --user enable --now hyprsnip.service");
                }
                ServiceCmd::Uninstall => {
                    println!("Run:\n  systemctl --user disable --now hyprsnip.service\n  rm ~/.config/systemd/user/hyprsnip.service\n  systemctl --user daemon-reload");
                }
            }
            Ok(())
        }
    }
}

fn effective_trim_options(mut base: TrimOptions, args: TrimArgs) -> TrimOptions {
    if let Some(level) = args.aggressiveness {
        base.aggressiveness = level.into();
    }

    if args.keep_blank_lines {
        base.keep_blank_lines = true;
    }
    if args.no_keep_blank_lines {
        base.keep_blank_lines = false;
    }

    if args.remove_box_drawing {
        base.remove_box_drawing = true;
    }
    if args.no_remove_box_drawing {
        base.remove_box_drawing = false;
    }

    if let Some(max_auto_lines) = args.max_auto_lines {
        base.max_auto_lines = max_auto_lines;
    }

    base
}

fn systemd_user_unit() -> &'static str {
    "[Unit]\nDescription=hyprsnip clipboard trimmer\n\n[Service]\nExecStart=hyprsnip daemon\nRestart=on-failure\n\n[Install]\nWantedBy=default.target\n"
}
