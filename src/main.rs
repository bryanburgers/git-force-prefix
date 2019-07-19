extern crate clap;
extern crate rayon;
extern crate sha1;

mod commit;
mod search;

use clap::{App, Arg};
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
    let matches = App::new("git force-prefix")
        .author("Bryan Burgers <bryan@burgers.io>")
        .version("0.1.0")
        .about("Force a commit hash to have a given prefix")
        .arg(
            Arg::with_name("prefix")
                .help("The hexidecimal prefix to calculate")
                .required(true)
                .validator(|hex| {
                    Search::parse(&hex).map_err(|search::SearchError { ch, pos }| {
                        format!("In '{}', the character '{}' at position {} is not a hexidecimal character.", hex, ch as char, pos + 1)
                    })?;
                    Ok(())
                }),
        )
        .get_matches();

    // Both of these unwraps are safe because the argument processor already validated that prefix
    // exists and can be successfully parsed by Search::parse.
    let search = Search::parse(matches.value_of("prefix").unwrap()).unwrap();

    // Get HEAD's commit blob
    let output = Command::new("git")
        .args(&["cat-file", "commit", "HEAD"])
        .output()
        .map_err(|_| ApplicationError::GitCatFileFailed)?;
    let output = String::from_utf8(output.stdout).map_err(|_| ApplicationError::CommitNotUTF8)?;

    // And parse it into something we can use
    let commit = Commit::parse(&output).map_err(|_| ApplicationError::CommitParseFailed)?;

    // Calculate a NEW commit that matches the prefix that we want. This runs forever until it
    // succeeds.
    let new_commit = force_prefix(&commit, &search);

    // We've found a new commit that will make this commit match the prefix! Because we only mess
    // with committer_timestamp and author_timestamp, we need to amend the current commit with
    // these new values.
    println!(
        "GIT_COMMITTER_DATE=\"{} {}\" git commit --date=\"{} {}\" --amend --no-edit",
        new_commit.committer_timestamp,
        new_commit.committer_timezone,
        new_commit.author_timestamp,
        new_commit.author_timezone
    );

    Ok(())
}

/// Find a new commit blob, based on the given one, that has a commit hash that matches the search.
fn force_prefix(commit: &Commit, search: &Search) -> Commit {
    // First, pre-create as much of the SHA1 hash and the constituent parts as possible.
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

    // Keep incrementing the committer timestamp until we can find a commit that matches...
    while !found {
        let i = iter.next().unwrap();

        // Search (in parallel) as many commits as possible, where the author timestamp is between
        // the original commit's author timestamp and the new committer timestamp
        let parallel_iterator = (0..(i + 1)).into_par_iter();
        let result = parallel_iterator.find_any(|j| {
            let author_timestamp = commit.author_timestamp + j;
            let committer_timestamp = commit.author_timestamp + i;
            // If we used these timestamps, what would the commit hash be?
            let h =
                calculate_hash_predigest(m.clone(), author_timestamp, b, committer_timestamp, c);
            // Does that commit hash match?
            let f = search.test(&h);
            if f {
                // Yay! We found one! Let's write out the bytes.
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
        */
    }

    // New commit is exactly the same as the old, except with its timestamps changed.
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

/// List of potential errors that we can run into
enum ApplicationError {
    /// We couldn't get the current commit blob
    GitCatFileFailed,
    /// The commit wasn't UTF-8 (WHO DOES THIS!?)
    CommitNotUTF8,
    /// We couldn't parse the commit blob
    CommitParseFailed,
}

impl ApplicationError {
    fn output_and_exit_code(&self) -> i32 {
        match *self {
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
        }
    }
}
