mod app;
mod tui;

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use meme_garden_core::{Metrics, SimConfig, Simulation};

#[derive(Debug, Parser)]
#[command(name = "meme-garden", version, about = "Meme Garden — memetic petri dish")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Run the interactive TUI.
    Run {
        #[arg(long, default_value = "configs/default.toml")]
        config: PathBuf,
        #[arg(long)]
        seed: Option<u64>,
        /// Initial ticks per second.
        #[arg(long, default_value_t = 10.0)]
        tps: f32,
    },
    /// Run headless and dump per-tick metrics as CSV to stdout.
    Headless {
        #[arg(long, default_value = "configs/default.toml")]
        config: PathBuf,
        #[arg(long)]
        seed: Option<u64>,
        #[arg(long, default_value_t = 500)]
        ticks: u64,
    },
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Run { config, seed, tps } => run_tui(config, seed, tps),
        Cmd::Headless { config, seed, ticks } => run_headless(config, seed, ticks),
    }
}

fn load_config(path: &std::path::Path) -> Result<SimConfig> {
    SimConfig::from_path(path).with_context(|| format!("loading config from {}", path.display()))
}

fn run_headless(config: PathBuf, seed: Option<u64>, ticks: u64) -> Result<()> {
    let cfg = load_config(&config)?;
    let mut sim = Simulation::new(cfg, seed);
    println!("{}", Metrics::csv_header());
    for _ in 0..ticks {
        let m = sim.step();
        println!("{}", m.to_csv_row());
    }
    Ok(())
}

fn run_tui(config: PathBuf, seed: Option<u64>, tps: f32) -> Result<()> {
    let cfg = load_config(&config)?;
    let sim = Simulation::new(cfg, seed);
    let mut app = app::App::new(sim, tps);
    tui::run(&mut app)
}
