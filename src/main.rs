mod api;
mod commands;
mod dashboard;
mod util;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "venice", version, about = "Venice AI CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show current USD and DIEM balances
    Balance,
    /// List available models
    Models {
        #[arg(
            long = "type",
            default_value = "image",
            value_parser = ["image", "text", "audio", "music"]
        )]
        model_type: String,
        #[arg(long, short)]
        verbose: bool,
    },
    /// Interactive catalog dashboard
    #[command(alias = "dash")]
    Dashboard {
        #[arg(long = "type", default_value = "all")]
        model_type: String,
        #[arg(long = "in", default_value = "any")]
        modality_in: String,
        #[arg(long = "out", default_value = "any")]
        modality_out: String,
        #[arg(long, default_value = "all")]
        vendor: String,
        #[arg(long, default_value = "all")]
        tag: String,
        #[arg(long, default_value = "all")]
        privacy: String,
        #[arg(long, default_value = "price")]
        sort: String,
        #[arg(long)]
        desc: bool,
        #[arg(long, default_value = "")]
        search: String,
    },
    /// Generate an image
    Image {
        prompt: String,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        width: Option<u32>,
        #[arg(long)]
        height: Option<u32>,
        #[arg(long = "aspect-ratio")]
        aspect_ratio: Option<String>,
        #[arg(long)]
        resolution: Option<String>,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long = "no-open")]
        no_open: bool,
    },
    /// Analyze an image with a vision model
    Vision {
        image: Option<PathBuf>,
        #[arg(long)]
        prompt: Option<String>,
        #[arg(long)]
        model: Option<String>,
        #[arg(long = "list-models")]
        list_models: bool,
        #[arg(long, short)]
        verbose: bool,
    },
    /// Transcribe or analyze a video via chat completions
    Video {
        video: Option<String>,
        #[arg(long)]
        prompt: Option<String>,
        #[arg(long)]
        model: Option<String>,
        #[arg(long = "list-models")]
        list_models: bool,
        #[arg(long, short)]
        verbose: bool,
    },
    /// Generate speech from text
    Tts {
        text: String,
        #[arg(long, default_value = "CalmWoman")]
        voice: String,
        #[arg(long)]
        model: Option<String>,
        #[arg(
            long,
            default_value = "mp3",
            value_parser = ["mp3", "wav", "opus", "aac", "flac"]
        )]
        format: String,
        #[arg(long)]
        speed: Option<f64>,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long = "no-play")]
        no_play: bool,
    },
    /// Search the web
    Search {
        query: String,
        #[arg(long, value_parser = ["brave", "google", "exa"])]
        provider: Option<String>,
        #[arg(long, default_value_t = 10)]
        limit: u8,
        #[arg(long)]
        json: bool,
    },
    /// Scrape a URL into text
    Scrape {
        url: String,
        #[arg(long, value_parser = ["markdown", "html", "text"])]
        format: Option<String>,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Ask a Grok model with xAI X/Twitter search
    Xsearch {
        query: String,
        #[arg(long, default_value = "grok-4-20")]
        model: String,
        #[arg(long)]
        json: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    if let Err(err) = run(cli.command) {
        eprintln!("Error: {err:#}");
        std::process::exit(1);
    }
}

fn run(command: Command) -> anyhow::Result<()> {
    match command {
        Command::Balance => commands::balance::run(),
        Command::Models {
            model_type,
            verbose,
        } => commands::models::run(&model_type, verbose),
        Command::Dashboard {
            model_type,
            modality_in,
            modality_out,
            vendor,
            tag,
            privacy,
            sort,
            desc,
            search,
        } => dashboard::run(dashboard::Args {
            model_type,
            modality_in,
            modality_out,
            vendor,
            tag,
            privacy,
            sort,
            desc,
            search,
        }),
        Command::Image {
            prompt,
            model,
            width,
            height,
            aspect_ratio,
            resolution,
            out,
            no_open,
        } => commands::image::run(commands::image::Args {
            prompt,
            model,
            width,
            height,
            aspect_ratio,
            resolution,
            out,
            no_open,
        }),
        Command::Vision {
            image,
            prompt,
            model,
            list_models,
            verbose,
        } => commands::vision::run(image, prompt, model, list_models, verbose),
        Command::Video {
            video,
            prompt,
            model,
            list_models,
            verbose,
        } => commands::video::run(video, prompt, model, list_models, verbose),
        Command::Tts {
            text,
            voice,
            model,
            format,
            speed,
            out,
            no_play,
        } => commands::tts::run(commands::tts::Args {
            text,
            voice,
            model,
            format,
            speed,
            out,
            no_play,
        }),
        Command::Search {
            query,
            provider,
            limit,
            json,
        } => commands::search::run(&query, provider.as_deref(), limit, json),
        Command::Scrape {
            url,
            format,
            out,
            json,
        } => commands::scrape::run(&url, format.as_deref(), out.as_deref(), json),
        Command::Xsearch {
            query,
            model,
            json,
        } => commands::xsearch::run(&query, &model, json),
    }
}
