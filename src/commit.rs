/// A struct that holds the things that we care about about a commit.
#[derive(Debug, Clone)]
pub struct Commit {
    /// We don't care what comes before author/committer. Includes the newline.
    pub preamble: String,
    /// The author's name and email
    pub author: String,
    /// The author timestamp. This is what we will twiddle to create new commit hashes.
    pub author_timestamp: i64,
    /// The textual timezone (we don't care about this really)
    pub author_timezone: String,
    /// The committer's name and email
    pub committer: String,
    /// The committer timestamp. This is what we will twiddle to create new commit hashes.
    pub committer_timestamp: i64,
    /// The textual timezone (we don't care about this really)
    pub committer_timezone: String,
    /// The commit message itself. This typically includes the trailing newline.
    pub message: String,
}

/// Empty struct that represents that we failed to parse the commit.
#[derive(Debug)]
pub struct CommitError;

impl Commit {
    /// Parse a string into a commit object.
    pub fn parse(commit: &str) -> Result<Commit, CommitError> {
        let mut i = commit.splitn(2, "\n\n");
        let header = i.next().unwrap();
        let message = i.next().unwrap().to_string();

        let mut preamble = String::new();
        let mut author = String::new();
        let mut author_timestamp = 0;
        let mut author_tz = String::new();
        let mut committer = String::new();
        let mut committer_timestamp = 0;
        let mut committer_tz = String::new();

        for line in header.lines() {
            if line.starts_with("author") {
                let mut i = line.rsplitn(3, ' ');
                author_tz = i.next().unwrap().into();
                let timestamp = i.next().unwrap();
                author = i.next().unwrap().into();
                author = author.chars().skip(7).collect();
                author_timestamp = timestamp.parse().unwrap();
            } else if line.starts_with("committer") {
                let mut i = line.rsplitn(3, ' ');
                committer_tz = i.next().unwrap().into();
                let timestamp = i.next().unwrap();
                committer = i.next().unwrap().into();
                committer = committer.chars().skip(10).collect();
                committer_timestamp = timestamp.parse().unwrap();
            } else {
                preamble.push_str(line);
                preamble.push_str("\n");
            }
        }

        let commit = Commit {
            preamble: preamble,
            author: author,
            author_timestamp: author_timestamp,
            author_timezone: author_tz,
            committer: committer,
            committer_timestamp: committer_timestamp,
            committer_timezone: committer_tz,
            message: message,
        };

        Ok(commit)
    }
}

/// Test that parsing actually works!
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_with_parent() {
        let commit = r#"tree cb44699325a0f4d127979cc8ae82354dd7e80ac6
parent 30b08f0d64ab1b436713cbd43d6cd43dc0d967e3
author Bryan Burgers <bryan@burgers.io> 1524752605 -0500
committer Bryan Burgers <bryan@burgers.io> 1524753225 -0500

Test commit
"#;

        let commit = Commit::parse(commit).unwrap();

        assert_eq!(
            commit.preamble,
            r#"tree cb44699325a0f4d127979cc8ae82354dd7e80ac6
parent 30b08f0d64ab1b436713cbd43d6cd43dc0d967e3
"#
        );
        assert_eq!(commit.author, "Bryan Burgers <bryan@burgers.io>");
        assert_eq!(commit.author_timestamp, 1524752605);
        assert_eq!(commit.author_timezone, "-0500");
        assert_eq!(commit.committer, "Bryan Burgers <bryan@burgers.io>");
        assert_eq!(commit.committer_timestamp, 1524753225);
        assert_eq!(commit.committer_timezone, "-0500");
        assert_eq!(commit.message, "Test commit\n");
    }

    #[test]
    fn parse_initial_commit() {
        let commit = r#"tree f7b61169107fb3b4262406b998df7cba3a379bd6
author Bryan Burgers <bryan@burgers.io> 1524680608 -0500
committer Bryan Burgers <bryan@burgers.io> 1524680608 -0500

Initial commit
"#;

        let commit = Commit::parse(commit).unwrap();

        assert_eq!(
            commit.preamble,
            "tree f7b61169107fb3b4262406b998df7cba3a379bd6\n"
        );
        assert_eq!(commit.author, "Bryan Burgers <bryan@burgers.io>");
        assert_eq!(commit.author_timestamp, 1524680608);
        assert_eq!(commit.author_timezone, "-0500");
        assert_eq!(commit.committer, "Bryan Burgers <bryan@burgers.io>");
        assert_eq!(commit.committer_timestamp, 1524680608);
        assert_eq!(commit.committer_timezone, "-0500");
        assert_eq!(commit.message, "Initial commit\n");
    }
}
