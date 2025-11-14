use clap::{Parser, Subcommand};

use crate::diff::diff_commits;
mod diff;
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
    Changes,
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
        Some(Commands::Changes) => {
            let commits_res = repo.get_commits();

            if commits_res.is_err() {
                eprintln!("Error getting commits: {}", commits_res.err().unwrap());
                return;
            }
            let mut lines_added: i64 = 0;
            let mut lines_removed: i64 = 0;

            let commits = commits_res.unwrap();

            for (i, commit) in commits.iter().enumerate() {
                if i > 0 {
                    let last_commit = &commits[i - 1];
                    let diff_res =
                        diff_commits(&commit, last_commit, &|h: &str| repo.get_object(h));

                    if diff_res.is_err() {
                        println!(
                            "Could not get diff between commit {} and {}",
                            last_commit.hash, commit.hash
                        );
                        continue;
                    }

                    let diff = diff_res.unwrap();

                    lines_added += diff.added as i64;
                    lines_removed += diff.removed as i64;
                }
            }

            println!("Lines added: {}", lines_added);
            println!("Lines removed: {}", lines_removed);
            println!("Lines total: {}", lines_added - lines_removed);
        }
        None => {}
    }
}
