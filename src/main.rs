mod app;
mod glitzer;

use app::App;
use clap::Parser;
use glitzer::repo::read_repo;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long, default_value = ".")]
    repo: String,
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let args = Cli::parse();

    let repo_result = read_repo(&args.repo);

    if repo_result.is_err() {
        eprintln!("Error reading repository: {}", repo_result.err().unwrap());
        return Ok(());
    }

    let repo = repo_result.unwrap();
    let mut app = App::new(repo)?;
    return Ok(ratatui::run(|terminal| app.run(terminal))?);
}
