use crate::model::{Author, Commit};
use chrono::prelude::*;
use nom::Err;
use nom::IResult;
use nom::bytes::complete::{tag, take_until};
use nom::character::complete::{hex_digit1, newline};
use nom::error::Error;

pub fn tree(input: &str) -> IResult<&str, &str> {
    let (input, _) = tag("tree ")(input)?;
    hash(input)
}

pub fn parent(input: &str) -> IResult<&str, &str> {
    let (input, _) = tag("parent ")(input)?;
    hash(input)
}

fn hash(input: &str) -> IResult<&str, &str> {
    hex_digit1(input)
}

fn author<'a, 'b>(input: &'a str, author_tag: &'b str) -> IResult<&'a str, Author> {
    let (input, _) = tag(author_tag)(input)?;
    let (input, name) = take_until(" <")(input)?;
    let (input, _) = tag(" <")(input)?;
    let (input, email) = take_until("> ")(input)?;
    let (input, _) = tag("> ")(input)?;
    Ok((
        input,
        Author {
            name: name.to_string(),
            email: email.to_string(),
        },
    ))
}

fn timestamp(input: &str) -> IResult<&str, &str> {
    let (input, ts_str) = take_until("\n")(input)?;
    return Ok((input, ts_str));
}

fn parse_timestamp(ts_str: &str) -> Result<DateTime<FixedOffset>, String> {
    DateTime::parse_from_str(ts_str, "%s %z")
        .map_err(|err| format!("Failed to parse timestamp from {}: {}", ts_str, err))
}

pub fn parse_commit(hash: String, input: &str) -> Result<Commit, String> {
    let (input, commit_tree) = tree(input).map_err(|err| err.to_string())?;
    let (input, _) = newline(input).map_err(|err: Err<Error<&str>>| err.to_string())?;
    let (input, commit_parent) = parent(input).map_err(|err| err.to_string())?;
    let (input, _) = newline(input).map_err(|err: Err<Error<&str>>| err.to_string())?;
    let (input, commit_author) = author(input, "author ").map_err(|err| err.to_string())?;
    let (input, ts_str) = timestamp(input).map_err(|err| err.to_string())?;

    let author_dt = parse_timestamp(ts_str)?;

    let (input, _) = newline(input).map_err(|err: Err<Error<&str>>| err.to_string())?;

    let (input, comitter) = author(input, "committer ").map_err(|err| err.to_string())?;
    let (input, ts_str) = timestamp(input).map_err(|err| err.to_string())?;

    let committed_at = parse_timestamp(ts_str)?;

    let (input, _) = newline(input).map_err(|err: Err<Error<&str>>| err.to_string())?;
    let (input, _) = newline(input).map_err(|err: Err<Error<&str>>| err.to_string())?;

    Ok(Commit {
        tree: commit_tree.to_string(),
        parent: commit_parent.to_string(),
        author: commit_author,
        authored_at: author_dt.to_utc(),
        committer: comitter,
        committed_at: committed_at.to_utc(),
        hash: hash,
        message: input.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_commit() {
        let commit_str = b"tree f170a88dea001046a4705aa4728c7d2fb48238b1\nparent fe013499538f359bb0c8d9ec204f9f96d7d3d372\nauthor Johannes Herrmann <johannes.r.herrmann@gmail.com> 1761384503 +0200\ncommitter Johannes Herrmann <johannes.r.herrmann@gmail.com> 1761384503 +0200\n\nRead Repository and objects\n";
        let commit_res = parse_commit(
            "c0ffee".to_string(),
            std::str::from_utf8(commit_str).unwrap(),
        );

        if !commit_res.is_ok() {
            println!("Error: {}", commit_res.err().unwrap());
            assert_eq!(true, false);
            return;
        }

        // assert_eq!(commit_res.err(), None);
        let commit = commit_res.unwrap();
        assert_eq!(commit.hash, "c0ffee".to_string());
        assert_eq!(
            commit.tree,
            "f170a88dea001046a4705aa4728c7d2fb48238b1".to_string()
        );
        assert_eq!(
            commit.parent,
            "fe013499538f359bb0c8d9ec204f9f96d7d3d372".to_string()
        );
        assert_eq!(commit.author.name, "Johannes Herrmann".to_string());
        assert_eq!(
            commit.author.email,
            "johannes.r.herrmann@gmail.com".to_string()
        );
        assert_eq!(
            commit.authored_at,
            DateTime::parse_from_rfc3339("2025-10-25T11:28:23+02:00")
                .unwrap()
                .with_timezone(&Utc)
        );
        assert_eq!(commit.message, "Read Repository and objects\n".to_string());
    }
}
