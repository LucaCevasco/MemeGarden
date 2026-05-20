mod app;
mod export;
mod runner;
mod tui;

use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use clap::{ArgGroup, Parser, Subcommand};
use meme_garden_core::{Metrics, SimConfig, Simulation};

const PRESET_DIR: &str = "configs/presets";

#[derive(Debug, Parser)]
#[command(
    name = "meme-garden",
    version,
    about = "Meme Garden — memetic petri dish"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Run the interactive TUI.
    #[command(group = ArgGroup::new("source").required(false).args(["config", "preset"]))]
    Run {
        #[arg(long, default_value = "configs/default.toml")]
        config: PathBuf,
        #[arg(long)]
        preset: Option<String>,
        #[arg(long)]
        seed: Option<u64>,
        #[arg(long)]
        run_id: Option<String>,
        /// Initial ticks per second.
        #[arg(long, default_value_t = 10.0)]
        tps: f32,
    },
    /// Run headless and write artifacts under runs/<run-id>/.
    #[command(group = ArgGroup::new("source").required(false).args(["config", "preset"]))]
    Headless {
        #[arg(long, default_value = "configs/default.toml")]
        config: PathBuf,
        #[arg(long)]
        preset: Option<String>,
        #[arg(long)]
        seed: Option<u64>,
        #[arg(long)]
        ticks: Option<u32>,
        #[arg(long)]
        run_id: Option<String>,
    },
    /// List shipped preset configs under configs/presets/.
    ListPresets,
    /// Re-emit metrics from a finished run into an alternative shape.
    Export {
        run_dir: PathBuf,
        #[arg(long, value_parser = ["csv", "jsonl", "summary-md"])]
        to: String,
    },
    /// Summarize a finished run with the active RunAnalyst provider.
    Analyze { run_dir: PathBuf },
    /// Translate a natural-language prompt into a SimConfig via ExperimentDesigner.
    Experiment {
        #[command(subcommand)]
        sub: ExperimentCmd,
    },
}

#[derive(Debug, Subcommand)]
enum ExperimentCmd {
    /// Map natural-language to a config. Without a real provider this exits 2.
    Design {
        prompt: String,
        #[arg(long)]
        out: Option<PathBuf>,
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
        Cmd::Run {
            config,
            preset,
            seed,
            run_id,
            tps,
        } => run_tui(config, preset, seed, run_id, tps),
        Cmd::Headless {
            config,
            preset,
            seed,
            ticks,
            run_id,
        } => run_headless(config, preset, seed, ticks, run_id),
        Cmd::ListPresets => list_presets(),
        Cmd::Export { run_dir, to } => export_cmd(run_dir, to),
        Cmd::Analyze { run_dir } => analyze_cmd(run_dir),
        Cmd::Experiment {
            sub: ExperimentCmd::Design { prompt, out },
        } => experiment_design(prompt, out),
    }
}

fn resolve_config(config: PathBuf, preset: Option<String>) -> Result<(SimConfig, String)> {
    let (path, short_name) = if let Some(name) = preset {
        let p = PathBuf::from(PRESET_DIR).join(format!("{name}.toml"));
        if !p.exists() {
            return Err(anyhow!("preset not found: {}", p.display()));
        }
        let stem = name;
        (p, stem)
    } else {
        let stem = config
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("default")
            .to_string();
        (config, stem)
    };
    let cfg = SimConfig::from_path(&path).with_context(|| format!("loading {}", path.display()))?;
    Ok((cfg, short_name))
}

fn run_tui(
    config: PathBuf,
    preset: Option<String>,
    seed: Option<u64>,
    run_id: Option<String>,
    tps: f32,
) -> Result<()> {
    let (mut cfg, short_name) = resolve_config(config, preset)?;
    cfg.validate().with_context(|| "validating config")?;
    if let Some(s) = seed {
        cfg.run.seed = s;
    }
    let run_id = run_id.unwrap_or_else(|| runner::default_run_id(&short_name));
    let mut writer = export::RunWriter::create(&run_id, &cfg)
        .with_context(|| format!("creating run dir for {run_id}"))?;
    let sim = Simulation::new(cfg.clone(), seed);
    let mut app = app::App::new(sim, tps);
    // Header line first.
    app.sim.emit_header(run_id.clone());
    let header_events = app.sim.events_drain();
    for e in &header_events {
        writer.write_event(e)?;
    }
    let result = tui::run(&mut app);
    // Persist whatever metrics the TUI accumulated.
    for m in &app.history {
        writer.write_summary_row(m)?;
        // Re-create a Tick event so the JSONL has the same shape as headless.
        // (This mirrors what runner::run_to_horizon emits.)
        writer.write_event(&meme_garden_core::Event::Tick(Box::new(m.clone())))?;
    }
    writer.finalize()?;
    result
}

fn run_headless(
    config: PathBuf,
    preset: Option<String>,
    seed: Option<u64>,
    ticks: Option<u32>,
    run_id: Option<String>,
) -> Result<()> {
    let (mut cfg, short_name) = resolve_config(config, preset)?;
    cfg.validate().with_context(|| "validating config")?;
    if let Some(s) = seed {
        cfg.run.seed = s;
    }
    if let Some(t) = ticks {
        cfg.run.horizon = t;
    }
    let horizon = cfg.run.horizon;
    let run_id = run_id.unwrap_or_else(|| runner::default_run_id(&short_name));
    let mut writer = export::RunWriter::create(&run_id, &cfg)
        .with_context(|| format!("creating run dir for {run_id}"))?;
    let mut sim = Simulation::new(cfg, seed);
    runner::run_to_horizon(&mut sim, &mut writer, horizon, &run_id, None)?;
    writer.finalize()?;
    eprintln!("wrote runs/{run_id}/");
    Ok(())
}

fn list_presets() -> Result<()> {
    let dir = std::fs::read_dir(PRESET_DIR);
    if dir.is_err() {
        println!("(no presets directory at {PRESET_DIR})");
        return Ok(());
    }
    let mut entries: Vec<_> = std::fs::read_dir(PRESET_DIR)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "toml").unwrap_or(false))
        .collect();
    entries.sort_by_key(|e| e.path());
    for entry in entries {
        let path = entry.path();
        let stem = path.file_stem().unwrap().to_string_lossy().to_string();
        let desc = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| {
                s.lines()
                    .next()
                    .filter(|l| l.starts_with("# description:"))
                    .map(|l| l.trim_start_matches("# description:").trim().to_string())
            })
            .unwrap_or_default();
        println!("{stem:<40}  {desc}");
    }
    Ok(())
}

fn export_cmd(run_dir: PathBuf, to: String) -> Result<()> {
    match to.as_str() {
        "csv" => export::regenerate_summary_csv(&run_dir)?,
        "jsonl" => export::validate_jsonl(&run_dir)?,
        "summary-md" => {
            let md = export::summarize_markdown(&run_dir)?;
            println!("{md}");
        }
        other => return Err(anyhow!("unknown export target: {other}")),
    }
    Ok(())
}

fn analyze_cmd(run_dir: PathBuf) -> Result<()> {
    let md = export::summarize_markdown(&run_dir)?;
    println!("{md}");
    Ok(())
}

fn experiment_design(_prompt: String, _out: Option<PathBuf>) -> Result<()> {
    eprintln!("Error: ai provider not configured");
    std::process::exit(2);
}

#[allow(dead_code)]
fn _unused_metrics_helpers() {
    // Forces these to stay in the binary's public surface during refactors.
    let _ = Metrics::csv_header;
}
