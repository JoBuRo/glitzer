use clap::Parser;
mod glitzer;
mod model;
mod parser;

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

    let repo_result = glitzer::read_repo(&args.repo);

    if repo_result.is_err() {
        eprintln!("Error reading repository: {}", repo_result.err().unwrap());
        return;
    }

    let repo = repo_result.unwrap();

    println!("Loaded repo\n{:?}", repo);

    if let Some(object_hash) = args.object {
        match repo.get_object(&object_hash) {
            Some(object) => {
                println!("Found object:\n{:?}", object);
            }
            None => {
                println!("Object with hash {} not found in repository.", object_hash);
            }
        }
    }
}
