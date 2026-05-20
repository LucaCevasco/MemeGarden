use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use meme_garden_core::{Event, Metrics, SimConfig};
use serde::Deserialize;

/// Writes the three artifacts for a single run:
///   * `config.toml`     — resolved configuration, self-describing.
///   * `events.jsonl`    — line-delimited Event records, header first.
///   * `summary.csv`     — flat per-tick aggregate, header first.
pub struct RunWriter {
    #[allow(dead_code)]
    dir: PathBuf,
    events: BufWriter<File>,
    summary: BufWriter<File>,
    summary_header_written: bool,
}

impl RunWriter {
    pub fn create(run_id: &str, config: &SimConfig) -> Result<Self> {
        let dir = PathBuf::from("runs").join(run_id);
        if dir.exists() {
            return Err(anyhow!(
                "run id {run_id} already exists at {} — pass --run-id to disambiguate",
                dir.display()
            ));
        }
        fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        fs::write(dir.join("config.toml"), config.to_toml_string()?)
            .with_context(|| "writing resolved config.toml")?;

        let events = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(dir.join("events.jsonl"))?;
        let summary = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(dir.join("summary.csv"))?;
        Ok(Self {
            dir,
            events: BufWriter::new(events),
            summary: BufWriter::new(summary),
            summary_header_written: false,
        })
    }

    #[allow(dead_code)]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn write_event(&mut self, event: &Event) -> Result<()> {
        serde_json::to_writer(&mut self.events, event)?;
        self.events.write_all(b"\n")?;
        Ok(())
    }

    pub fn write_summary_row(&mut self, m: &Metrics) -> Result<()> {
        if !self.summary_header_written {
            writeln!(self.summary, "{}", Metrics::csv_header())?;
            self.summary_header_written = true;
        }
        writeln!(self.summary, "{}", m.to_csv_row())?;
        Ok(())
    }

    pub fn finalize(mut self) -> Result<()> {
        self.events.flush()?;
        self.summary.flush()?;
        // why: fsync so a kill -9'd run leaves a recoverable file. The trade-off
        // is a small per-run overhead, which is invisible at MVP scale.
        let inner = self.events.into_inner()?;
        inner.sync_all()?;
        let inner_s = self.summary.into_inner()?;
        inner_s.sync_all()?;
        Ok(())
    }
}

// ----- export subcommand support -----

/// Read every Tick event in `events.jsonl` and rewrite `summary.csv` from them.
pub fn regenerate_summary_csv(run_dir: &Path) -> Result<()> {
    let f = File::open(run_dir.join("events.jsonl"))
        .with_context(|| format!("opening {}/events.jsonl", run_dir.display()))?;
    let reader = BufReader::new(f);
    let out = File::create(run_dir.join("summary.csv"))?;
    let mut w = BufWriter::new(out);
    writeln!(w, "{}", Metrics::csv_header())?;
    for line in reader.lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        // Only Tick records become summary rows.
        let v: serde_json::Value = serde_json::from_str(&line)?;
        if v.get("kind").and_then(|k| k.as_str()) != Some("tick") {
            continue;
        }
        // The Tick variant is `Tick(Box<Metrics>)` with #[serde(tag="kind")] so
        // the inner fields are inlined alongside "kind". Strip kind and decode.
        let mut v = v;
        if let Some(obj) = v.as_object_mut() {
            obj.remove("kind");
        }
        let m: Metrics = serde_json::from_value(v)?;
        writeln!(w, "{}", m.to_csv_row())?;
    }
    Ok(())
}

/// Validate that every JSONL line is parseable as a known Event kind.
pub fn validate_jsonl(run_dir: &Path) -> Result<()> {
    let f = File::open(run_dir.join("events.jsonl"))?;
    let reader = BufReader::new(f);
    let mut n = 0u64;
    for (i, line) in reader.lines().enumerate() {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        let v: serde_json::Value = serde_json::from_str(&line)
            .with_context(|| format!("line {}: not valid JSON", i + 1))?;
        let kind = v
            .get("kind")
            .and_then(|x| x.as_str())
            .ok_or_else(|| anyhow!("line {}: missing 'kind'", i + 1))?;
        match kind {
            "header" | "tick" | "birth" | "death" | "transmission" | "mutation"
            | "recombination" | "meme_forgotten" | "extinction" | "cluster_snapshot" => {}
            other => return Err(anyhow!("line {}: unknown event kind '{other}'", i + 1)),
        }
        n += 1;
    }
    println!("ok — {n} records");
    Ok(())
}

/// Read `events.jsonl`, rebuild the metrics history, and produce a Markdown
/// summary using the active RunAnalyst (NoopProvider in the MVP).
pub fn summarize_markdown(run_dir: &Path) -> Result<String> {
    use meme_garden_core::ai::{NoopProvider, RunAnalyst};
    use meme_garden_core::LineageGraph;

    let history = load_metrics_history(run_dir)?;
    let lineage = LineageGraph::new();
    let prose = NoopProvider.summarize(&history, &lineage);
    let last = history
        .last()
        .ok_or_else(|| anyhow!("no Tick records in {}", run_dir.display()))?;

    let mut s = String::new();
    s.push_str(&format!("# Run summary — {}\n\n", run_dir.display()));
    s.push_str(&format!("- ticks recorded: {}\n", history.len()));
    s.push_str(&format!("- final alive: {}\n", last.alive));
    s.push_str(&format!("- final food: {}\n", last.food_count));
    s.push_str(&format!(
        "- final diversity (Shannon): {:.3}\n",
        last.diversity_shannon
    ));
    s.push_str(&format!(
        "- final top1 dominance: {:.1}%\n",
        last.dominance_top1_fraction * 100.0
    ));
    s.push_str(&format!(
        "- final cooperative prevalence: {:.1}%\n",
        last.meme_prevalence_by_kind.cooperative * 100.0
    ));
    s.push_str(&format!(
        "- final aggressive prevalence: {:.1}%\n",
        last.meme_prevalence_by_kind.aggressive * 100.0
    ));
    s.push_str("\n## RunAnalyst\n\n");
    s.push_str(&prose);
    s.push('\n');
    Ok(s)
}

fn load_metrics_history(run_dir: &Path) -> Result<Vec<Metrics>> {
    let f = File::open(run_dir.join("events.jsonl"))?;
    let reader = BufReader::new(f);
    let mut out = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        let v: serde_json::Value = serde_json::from_str(&line)?;
        if v.get("kind").and_then(|k| k.as_str()) != Some("tick") {
            continue;
        }
        let mut v = v;
        if let Some(obj) = v.as_object_mut() {
            obj.remove("kind");
        }
        let m: Metrics = serde_json::from_value(v)?;
        out.push(m);
    }
    Ok(out)
}

/// Tiny helper kept here for the export-roundtrip integration test.
#[derive(Deserialize)]
#[allow(dead_code)]
pub(crate) struct HeaderProbe {
    pub kind: String,
    pub schema_version: u32,
    pub run_id: String,
}
