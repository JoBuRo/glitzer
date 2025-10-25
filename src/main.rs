mod glitzer;
mod model;

fn main() {
    match glitzer::read_object(".git/objects/80/ff239150a83e68c452ff7b33238e121799b99a") {
        Ok(object) => {
            println!("Object parsed successfully:");
            println!("{:?}", object);
        }
        Err(e) => {
            println!("Failed to parse object: {}", e);
        }
    }

    match glitzer::read_repo(".") {
        Ok(repo) => {
            println!("Repository read successfully:");
            println!("{:?}", repo);
        }
        Err(e) => {
            println!("Failed to read repository: {}", e);
        }
    }
}
