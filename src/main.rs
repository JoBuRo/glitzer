use clap::Parser;
mod git_objects;
mod glitzer;
mod parser;
mod repo;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = ".")]
    repo: String,

    #[arg(short, long)]
    object: Option<String>,
}

fn main() {
    let args = Args::parse();

    let repo_result = glitzer::get_repo(&args.repo);

    if repo_result.is_err() {
        eprintln!("Error reading repository: {}", repo_result.err().unwrap());
        return;
    }

    let repo = repo_result.unwrap();

    println!("{:?}\n", repo);

    if let Some(object_hash) = args.object {
        let object_res = repo.get_object(&object_hash);

        if object_res.is_err() {
            eprintln!(
                "Error getting object {}: {}",
                object_hash,
                object_res.err().unwrap()
            );

            let raw_object_res = repo.get_raw_object(&object_hash);

            if raw_object_res.is_err() {
                eprintln!(
                    "Error getting raw object {}: {}",
                    object_hash,
                    raw_object_res.err().unwrap()
                );
                return;
            }

            println!("Raw Object {}:\n{:?}", object_hash, raw_object_res.unwrap());
            return;
        }

        let object = object_res.unwrap();
        println!("Object {}:\n{:?}", object_hash, object);
        return;
    }

    let commits_res = repo.get_commits();

    if commits_res.is_err() {
        eprintln!("Error getting commits: {}", commits_res.err().unwrap());
        return;
    }

    for commit in commits_res.unwrap() {
        println!("{:?}\n", commit);
    }
}
