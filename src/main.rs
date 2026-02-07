mod app;
mod glitzer;

use app::App;
use clap::Parser;
use color_eyre::Result;
use glitzer::repo::read_repo;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long, default_value = ".")]
    repo: String,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Cli::parse();

    let repo = read_repo(&args.repo)?;

    let mut app = App::new(repo)?;
    return Ok(ratatui::run(|terminal| app.run(terminal))?);
}
