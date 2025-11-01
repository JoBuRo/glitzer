use clap::{Parser, Subcommand};
mod git_objects;
mod glitzer;
mod parser;
mod repo;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long, default_value = ".")]
    repo: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Object { hash: String },
    History,
}

fn main() {
    let args = Cli::parse();

    let repo_result = glitzer::get_repo(&args.repo);

    if repo_result.is_err() {
        eprintln!("Error reading repository: {}", repo_result.err().unwrap());
        return;
    }

    let repo = repo_result.unwrap();

    println!("{:?}\n", repo);

    match &args.command {
        Some(Commands::Object { hash }) => {
            let object_res = repo.get_object(hash);

            if object_res.is_err() {
                eprintln!(
                    "Error getting object {}: {}",
                    hash,
                    object_res.err().unwrap()
                );
                return;
            }

            let object = object_res.unwrap();
            println!("Object {}:\n{:?}", &hash[0..7], object);
        }
        Some(Commands::History) => {
            let commits_res = repo.get_commits();

            if commits_res.is_err() {
                eprintln!("Error getting commits: {}", commits_res.err().unwrap());
                return;
            }

            for commit in commits_res.unwrap() {
                println!("{:?}\n", commit);
            }
        }
        None => {}
    }
}
