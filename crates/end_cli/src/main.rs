use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use end_io::{default_aic_toml, load_aic, load_catalog};
use end_opt::run_two_stage;
use end_report::{Lang, build_report};
use std::path::PathBuf;

const AIC_TOML_FILENAME: &str = "aic.toml";

#[derive(Debug, Parser)]
#[command(name = "end-cli", about = "v2 recipe optimization CLI (TOML-only)")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Clone, Subcommand)]
enum Command {
    /// Initialize `aic.toml` in current directory.
    Init {
        /// Overwrite existing file.
        #[arg(long)]
        force: bool,

        /// Path to output aic.toml.
        #[arg(long, value_name = "FILE", default_value = AIC_TOML_FILENAME)]
        aic: PathBuf,

        /// Directory containing items.toml / facilities.toml / recipes.toml.
        /// If omitted, builtin data embedded at compile time is used.
        #[arg(long, value_name = "DIR")]
        data_dir: Option<PathBuf>,
    },

    /// Solve optimization using v2 TOML inputs.
    Solve {
        /// Report language.
        #[arg(long, value_enum, default_value_t = Lang::Zh)]
        lang: Lang,

        /// Directory containing items.toml / facilities.toml / recipes.toml.
        /// If omitted, builtin data embedded at compile time is used.
        #[arg(long, value_name = "DIR")]
        data_dir: Option<PathBuf>,

        /// Path to aic.toml used by `solve`.
        #[arg(long, value_name = "FILE", default_value = AIC_TOML_FILENAME)]
        aic: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command.unwrap_or(Command::Solve {
        lang: Lang::Zh,
        data_dir: None,
        aic: PathBuf::from(AIC_TOML_FILENAME),
    }) {
        Command::Init {
            force,
            aic,
            data_dir,
        } => init_aic(force, aic, data_dir),
        Command::Solve {
            lang,
            data_dir,
            aic,
        } => solve(lang, data_dir, aic),
    }
}

fn init_aic(force: bool, aic: PathBuf, data_dir: Option<PathBuf>) -> Result<()> {
    let path = std::env::current_dir()
        .context("getting current dir")?
        .join(aic);
    if path.exists() && !force {
        anyhow::bail!(
            "{} already exists (use --force to overwrite)",
            path.display()
        );
    }

    let catalog = load_catalog(data_dir.as_deref()).context("loading catalog")?;
    let toml = default_aic_toml(&catalog).context("building default aic.toml")?;

    std::fs::write(&path, toml).with_context(|| format!("writing {}", path.display()))?;
    eprintln!("wrote {}", path.display());
    Ok(())
}

fn solve(lang: Lang, data_dir: Option<PathBuf>, aic_path: PathBuf) -> Result<()> {
    let catalog = load_catalog(data_dir.as_deref()).context("loading catalog")?;

    if !aic_path.exists() {
        let init_hint = format!("end-cli init --aic {}", aic_path.display());
        anyhow::bail!(
            "{} not found; run `{}` to create it",
            aic_path.display(),
            init_hint
        );
    }

    let aic =
        load_aic(&aic_path, &catalog).with_context(|| format!("loading {}", aic_path.display()))?;

    let solution = run_two_stage(&catalog, &aic).context("running optimization")?;
    let report = build_report(lang, &catalog, &aic, &solution).context("building report")?;

    println!("{}", report);
    Ok(())
}
