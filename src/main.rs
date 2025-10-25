use clap::Parser;
mod glitzer;
mod model;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = ".")]
    repo: String,
}

fn main() {
    let args = Args::parse();

    match glitzer::read_repo(&args.repo) {
        Ok(repo) => {
            println!("{:?}", repo);
        }
        Err(e) => {
            eprintln!("Error reading repository: {}", e);
        }
    }
}
