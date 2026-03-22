use clap::Parser;
use color_eyre::Result;
use glitzer::git::repo::Repository;
use glitzer::ui::app::App;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long, default_value = ".")]
    repo: String,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Cli::parse();

    let repo = Repository::new(args.repo)?;

    let mut app = App::new(repo)?;
    Ok(ratatui::run(|terminal| app.run(terminal))?)
}
