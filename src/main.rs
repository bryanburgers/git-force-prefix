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
    let arg = std::env::args().nth(1).unwrap_or("".to_string());
    if arg == "" {
        eprintln!("usage: git-prefix <hexstring>");
        exit(1);
    }

    let output = Command::new("git")
        .args(&["cat-file", "commit", "HEAD"])
        .output()
        .expect("Failed to call git");

    let output = match String::from_utf8(output.stdout) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("Git commit was not UTF-8");
            exit(1);
        }
    };

    let commit = Commit::parse(&output);
    let search = match Search::parse(&arg) {
        Ok(search) => search,
        Err(_) => {
            eprintln!("Invalid argument '{}'. Must be a hex string.", arg);
            exit(1);
        }
    };

    let new_commit = force_prefix(&commit, &search);

    println!(
        "GIT_COMMITTER_DATE=\"{} {}\" git commit --date=\"{} {}\" --amend --no-edit",
        new_commit.committer_timestamp,
        new_commit.committer_timezone,
        new_commit.author_timestamp,
        new_commit.author_timezone
    );
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
