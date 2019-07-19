/// A struct that holds the things that we care about about a commit.
#[derive(Debug, Clone, Copy)]
pub struct Commit<'a> {
    /// We don't care what comes before author/committer. Includes the newline.
    pub preamble: &'a str,
    /// The author's name and email
    pub author: &'a str,
    /// The author timestamp. This is what we will twiddle to create new commit hashes.
    pub author_timestamp: i64,
    /// The textual timezone (we don't care about this really)
    pub author_timezone: &'a str,
    /// The committer's name and email
    pub committer: &'a str,
    /// The committer timestamp. This is what we will twiddle to create new commit hashes.
    pub committer_timestamp: i64,
    /// The textual timezone (we don't care about this really)
    pub committer_timezone: &'a str,
    /// The commit message itself. This typically includes the trailing newline.
    pub message: &'a str,
}

/// Empty struct that represents that we failed to parse the commit.
#[derive(Debug)]
pub struct CommitError;

impl<'a> Commit<'a> {
    /// Parse a string into a commit object.
    pub fn parse(commit: &'a str) -> Result<Commit<'a>, CommitError> {
        let mut i = commit.splitn(2, "\n\n");
        let header = i.next().ok_or(CommitError)?;
        let message = i.next().ok_or(CommitError)?;

        // Split the header into 3 parts: the preamble, the author line, and the committer line.
        // The preamble can be "tree {hash}\nparent {hash}" or just "tree {hash}". Either way, we
        // just want everything up to the "author" line.
        let author_line_start_idx = header.find("author").ok_or(CommitError)?;
        let (preamble, rest) = header.split_at(author_line_start_idx);
        let author_line_end_idx = rest.find("\n").ok_or(CommitError)?;
        let (author_line, committer_line) = rest.split_at(author_line_end_idx);

        // Split "author Some Name <example@wherever.com> 1524680608 -0500" into three parts:
        // * "-0500"
        // * "1524680608"
        // * "author Some Name <example@wherever.com>"
        let mut i = author_line.rsplitn(3, ' ');
        let author_tz = i.next().ok_or(CommitError)?;
        let timestamp = i.next().ok_or(CommitError)?;
        let author_timestamp = timestamp.parse().map_err(|_| CommitError)?;
        // Strip off the "author " from the front.
        let author = &i.next().ok_or(CommitError)?[7..];

        // Split "committer Some Name <example@wherever.com> 1524680608 -0500" into three parts:
        // * "-0500"
        // * "1524680608"
        // * "committer Some Name <example@wherever.com>"
        let mut i = committer_line.rsplitn(3, ' ');
        let committer_tz = i.next().ok_or(CommitError)?;
        let timestamp = i.next().ok_or(CommitError)?;
        let committer_timestamp = timestamp.parse().map_err(|_| CommitError)?;
        // Strip off the "committer " from the front.
        let committer = &i.next().ok_or(CommitError)?[11..];

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
    fn parse_multiline_commit_message() {
        let commit = r#"tree cb44699325a0f4d127979cc8ae82354dd7e80ac6
parent 30b08f0d64ab1b436713cbd43d6cd43dc0d967e3
author Bryan Burgers <bryan@burgers.io> 1524752605 -0500
committer Bryan Burgers <bryan@burgers.io> 1524753225 -0500

This is the subject

This is the body. Note that there was a second double-newline there.
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
        assert_eq!(commit.message, "This is the subject\n\nThis is the body. Note that there was a second double-newline there.\n");
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
