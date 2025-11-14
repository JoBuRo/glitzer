use crate::git_objects::*;
use chrono::prelude::*;
use nom::Err;
use nom::IResult;
use nom::Parser;
use nom::bytes::complete::{tag, take, take_until};
use nom::character::complete::{digit1, hex_digit1, newline, space1};
use nom::combinator::opt;
use nom::error::Error;
use nom::error::ParseError;
use nom::multi::many1;

fn tree(input: &str) -> IResult<&str, &str> {
    let (input, _) = tag("tree ")(input)?;
    let (input, hash_value) = hash(input)?;
    let (input, _) = newline(input)?;
    Ok((input, hash_value))
}

fn parent(input: &str) -> IResult<&str, &str> {
    let (input, _) = tag("parent ")(input)?;
    let (input, hash_value) = hash(input)?;
    let (input, _) = newline(input)?;
    Ok((input, hash_value))
}

fn hash(input: &str) -> IResult<&str, &str> {
    hex_digit1(input)
}

fn author<'a>(input: &'a str, author_tag: &str) -> IResult<&'a str, Author> {
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

fn gpgsig(input: &str) -> IResult<&str, &str> {
    let (input, _) = tag("gpgsig ")(input)?;
    let (input, sig_block) = take_until("\n\n")(input)?;
    let (input, _) = tag("\n")(input)?;
    Ok((input, sig_block))
}

fn timestamp(input: &str) -> IResult<&str, &str> {
    let (input, ts_str) = take_until("\n")(input)?;
    let (input, _) = newline(input)?;
    Ok((input, ts_str))
}

fn tree_entry(input: &[u8]) -> IResult<&[u8], TreeEntry> {
    let (input, mode) = digit1(input)?;
    let (input, _) = space1(input)?;
    let (input, name) = take_until(&b"\0"[..])(input)?;
    let (input, _) = tag(&b"\0"[..])(input)?;
    let (input, hash_bytes) = take(20usize)(input)?; // 20 bytes for SHA-1

    let hash = hex::encode(hash_bytes);
    let mode_str = std::str::from_utf8(mode)
        .map_err(|_| Err::Error(Error::from_error_kind(input, nom::error::ErrorKind::Verify)))?;

    let name_str = std::str::from_utf8(name)
        .map_err(|_| Err::Error(Error::from_error_kind(input, nom::error::ErrorKind::Alpha)))?;

    let mode = match mode_str {
        "100644" => EntryMode::Text,
        "100755" => EntryMode::Exe,
        "120000" => EntryMode::Symlink,
        "40000" => EntryMode::Tree,
        "160000" => EntryMode::Gitlink,
        _ => {
            return Err(Err::Error(Error::from_error_kind(
                input,
                nom::error::ErrorKind::Verify,
            )));
        }
    };

    Ok((
        input,
        TreeEntry {
            mode,
            hash,
            name: name_str.to_string(),
        },
    ))
}

pub fn parse_tree(input: &[u8], hash: &str) -> Result<Tree, String> {
    let (input, mut entries) = many1(tree_entry)
        .parse(input)
        .map_err(|err| err.to_string())?;

    assert_eq!(
        input.len(),
        0,
        "Did not consume all input, rest: {:?}",
        input
    );

    entries.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(Tree {
        hash: hash.to_string(),
        entries,
    })
}

fn parse_timestamp(ts_str: &str) -> Result<DateTime<FixedOffset>, String> {
    DateTime::parse_from_str(ts_str, "%s %z")
        .map_err(|err| format!("Failed to parse timestamp from {}: {}", ts_str, err))
}

pub fn parse_commit(hash: String, input: &str) -> Result<Commit, String> {
    let (input, commit_tree) = tree(input).map_err(|err| err.to_string())?;
    let (input, commit_parent) = opt(parent).parse(input).map_err(|err| err.to_string())?;
    let (input, commit_author) = author(input, "author ").map_err(|err| err.to_string())?;
    let (input, ts_str) = timestamp(input).map_err(|err| err.to_string())?;

    let author_dt = parse_timestamp(ts_str)?;

    let (input, comitter) = author(input, "committer ").map_err(|err| err.to_string())?;
    let (input, ts_str) = timestamp(input).map_err(|err| err.to_string())?;

    let committed_at = parse_timestamp(ts_str)?;

    let (input, _) = opt(gpgsig).parse(input).map_err(|err| err.to_string())?;

    let (input, _) = newline(input).map_err(|err: Err<Error<&str>>| err.to_string())?;

    Ok(Commit {
        tree: commit_tree.to_string(),
        parent: commit_parent.map(|p| p.to_string()),
        author: commit_author,
        authored_at: author_dt.to_utc(),
        _committer: comitter,
        _committed_at: committed_at.to_utc(),
        hash,
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
            Some("fe013499538f359bb0c8d9ec204f9f96d7d3d372".to_string())
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

    #[test]
    fn test_parse_github_commit() {
        let commit_str = b"tree 8f57a99980891ccc68701b94b94342f7ae0e02d6\nauthor Joe <Johannes.R.Herrmann@gmail.com> 1761379929 +0200\ncommitter GitHub <noreply@github.com> 1761379929 +0200\ngpgsig -----BEGIN PGP SIGNATURE-----\n \n <cert> \n -----END PGP SIGNATURE-----\n \n\nInitial commit";
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
            "8f57a99980891ccc68701b94b94342f7ae0e02d6".to_string()
        );
        assert_eq!(commit.parent, None);
        assert_eq!(commit.author.name, "Joe".to_string());
        assert_eq!(
            commit.author.email,
            "Johannes.R.Herrmann@gmail.com".to_string()
        );
        assert_eq!(commit._committer.name, "GitHub".to_string());
        assert_eq!(commit._committer.email, "noreply@github.com".to_string());
        assert_eq!(
            commit.authored_at,
            DateTime::parse_from_rfc3339("2025-10-25T10:12:09+02:00")
                .unwrap()
                .with_timezone(&Utc)
        );
        assert_eq!(commit.message, "Initial commit".to_string());
    }

    #[test]
    fn test_parse_tree() {
        let tree_bytes = b"100644 .gitignore\0\xec\x1f\xa2\x087\xc3\x83\xc8\xf0\xb4\x98\x0e\xf7$#|\xd6\xcd\rC100644 Cargo.lock\0\xaa\xfe\xff\xcb|\x10>\xfc\x1aPu\xe0AX\xa7\x87eV\x95\x8a100644 Cargo.toml\0\xb4To\0Kd\x95\x9b\xa1\xe7\naMx\x90\xe9\xb4)\xf1\x92100644 LICENSE\0&\x1e\xeb\x9e\x9f\x8b+K\r\x11\x93f\xdd\xa9\x9co\xd7\xd3\\d40000 src\0\xf9\x85\xf1\x93\xba\x83,\xc1;\x9d|\xa7\x9b<\x1c6\x9cT\xe6=";

        let tree_res = parse_tree(tree_bytes, "c0ffee");

        if !tree_res.is_ok() {
            println!("Error: {}", tree_res.err().unwrap());
            assert_eq!(true, false);
            return;
        }

        let tree = tree_res.unwrap();

        assert_eq!(tree.hash, "c0ffee".to_string());
        assert_eq!(tree.entries.len(), 5);
        assert_eq!(tree.entries[0].mode, EntryMode::Text);
        assert_eq!(tree.entries[0].name, ".gitignore".to_string());
        assert_eq!(
            tree.entries[0].hash,
            "ec1fa20837c383c8f0b4980ef724237cd6cd0d43".to_string()
        );
    }
}
