extern crate rayon;
extern crate sha1;

mod commit;
mod search;

use commit::Commit;
use rayon::prelude::*;
use search::Search;
use std::fmt::Write;
use std::iter::Iterator;
use std::process::{exit, Command};

fn main() {
    let result = run();

    // If run() failed, handle it with the correct output and exit code.
    if let Some(err) = result.err() {
        let exit_code = err.output_and_exit_code();
        exit(exit_code);
    }
}

fn run() -> Result<(), ApplicationError> {
    let arg = std::env::args()
        .nth(1)
        .ok_or(ApplicationError::MissingPrefixArgument)?;

    let search = Search::parse(&arg).map_err(|search::SearchError { ch, pos }| {
        ApplicationError::InvalidPrefixArgument(arg, ch as char, pos)
    })?;

    let output = Command::new("git")
        .args(&["cat-file", "commit", "HEAD"])
        .output()
        .map_err(|_| ApplicationError::GitCatFileFailed)?;

    let output = String::from_utf8(output.stdout).map_err(|_| ApplicationError::CommitNotUTF8)?;

    let commit = Commit::parse(&output).map_err(|_| ApplicationError::CommitParseFailed)?;

    let new_commit = force_prefix(&commit, &search);

    println!(
        "GIT_COMMITTER_DATE=\"{} {}\" git commit --date=\"{} {}\" --amend --no-edit",
        new_commit.committer_timestamp,
        new_commit.committer_timezone,
        new_commit.author_timestamp,
        new_commit.author_timezone
    );

    Ok(())
}

fn force_prefix(commit: &Commit, search: &Search) -> Commit {
    let a = format!("{}author {} ", commit.preamble, commit.author);
    let a = a.as_bytes();
    let b = format!(
        " {}\ncommitter {} ",
        commit.author_timezone, commit.committer
    );
    let b = b.as_bytes();
    let c = format!(" {}\n\n{}", commit.committer_timezone, commit.message);
    let c = c.as_bytes();

    let mut iter = 0..;
    let mut found = false;

    let mut author_timestamp = 0;
    let mut committer_timestamp = 0;

    let len = a.len() + 10 + b.len() + 10 + c.len();

    let mut m = sha1::Sha1::new();
    m.update(b"commit ");
    m.update(len.to_string().as_bytes());
    m.update(b"\0");
    m.update(a);

    while !found {
        let i = iter.next().unwrap();

        let pi = (0..(i + 1)).into_par_iter();
        let result = pi.find_any(|j| {
            let author_timestamp = commit.author_timestamp + j;
            let committer_timestamp = commit.author_timestamp + i;
            let h =
                calculate_hash_predigest(m.clone(), author_timestamp, b, committer_timestamp, c);
            let f = search.test(&h);
            if f {
                let mut s = String::new();
                for &byte in h.iter() {
                    write!(&mut s, "{:02x}", byte).expect("Unable to write");
                }
                eprintln!("Found {}", s);
            }
            f
        });

        if let Some(j) = result {
            found = true;
            author_timestamp = commit.author_timestamp + j;
            committer_timestamp = commit.author_timestamp + i;
        }

        // Old, single-core code
        /*
        for j in 0..(i+1) {
            attempts += 1;

            author_timestamp = commit.author_timestamp + j;
            committer_timestamp = commit.author_timestamp + i;
            let h = calculate_hash(a, author_timestamp, b, committer_timestamp, c);
            if search.test(&h) {
                let mut s = String::new();
                for &byte in h.iter() {
                    write!(&mut s, "{:02x}", byte).expect("Unable to write");
                }
                eprintln!("Found {} after {} attempts", s, attempts);
                found = true;
                break
            }
        }
        */    }

    let mut new_commit = commit.clone();
    new_commit.author_timestamp = author_timestamp;
    new_commit.committer_timestamp = committer_timestamp;

    new_commit
}

// Code that goes slightly slower because it has to hash a little bit more
/*
#[inline]
fn calculate_hash(a: &[u8], author_timestamp: i64, b: &[u8], committer_timestamp: i64, c: &[u8]) -> [u8; 20] {
    let author_timestamp = author_timestamp.to_string();
    let author_timestamp = author_timestamp.as_bytes();
    let committer_timestamp = committer_timestamp.to_string();
    let committer_timestamp = committer_timestamp.as_bytes();

    let len = a.len() + author_timestamp.len() + b.len() + committer_timestamp.len() + c.len();

    let mut m = sha1::Sha1::new();
    m.update(b"commit ");
    m.update(len.to_string().as_bytes());
    m.update(b"\0");
    m.update(a);
    m.update(author_timestamp);
    m.update(b);
    m.update(committer_timestamp);
    m.update(c);

    let digest = m.digest();
    digest.bytes()
}
*/

#[inline]
fn calculate_hash_predigest(
    mut m: sha1::Sha1,
    author_timestamp: i64,
    b: &[u8],
    committer_timestamp: i64,
    c: &[u8],
) -> [u8; 20] {
    let author_timestamp = author_timestamp.to_string();
    let author_timestamp = author_timestamp.as_bytes();
    let committer_timestamp = committer_timestamp.to_string();
    let committer_timestamp = committer_timestamp.as_bytes();

    m.update(author_timestamp);
    m.update(b);
    m.update(committer_timestamp);
    m.update(c);

    let digest = m.digest();
    digest.bytes()
}

enum ApplicationError {
    MissingPrefixArgument,
    GitCatFileFailed,
    CommitNotUTF8,
    CommitParseFailed,
    InvalidPrefixArgument(String, char, usize),
}

impl ApplicationError {
    fn output_and_exit_code(&self) -> i32 {
        match *self {
            ApplicationError::MissingPrefixArgument => {
                eprintln!("usage: git-prefix <hexstring>");
                1
            }
            ApplicationError::GitCatFileFailed => {
                eprintln!("ERROR: Failed to call git. Is the current directory a repo?");
                1
            }
            ApplicationError::CommitNotUTF8 => {
                eprintln!("ERROR: The commit could not be parsed as UTF-8");
                1
            }
            ApplicationError::CommitParseFailed => {
                eprintln!("ERROR: Failed to parse the commit");
                1
            }
            ApplicationError::InvalidPrefixArgument(ref arg, ref ch, ref pos) => {
                eprintln!("Invalid argument '{}'. Character '{}' at position {} is not a hexidecimal character.", arg, ch, pos + 1);
                1
            }
        }
    }
}
