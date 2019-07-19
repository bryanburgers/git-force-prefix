`git force-prefix` alters the commit hash of the HEAD commit so that its prefix
is a specific value.

This program came about because I was in a debate with a coworker about whether
it is possible to have any control of the git hash of a commit.


## How it works

The commit hash for a commit is calculated by taking the SHA1 hash of a
specific set of bytes representing the commit. This can seen approximately
using `git cat-file commit HEAD`:

```
tree 35e2cf7b5ae7082675c96246740f5026ff6b796d
parent d06f00d1679a4a0fdf38bc4b95309514acd6011b
author Bryan Burgers <bryan@burgers.io> 1524851698 -0500
committer Bryan Burgers <bryan@burgers.io> 1524859906 -0500

Cleanup error handling
```

If we assume that we can't change any data in this structure, then my coworker
was right: we don't have any control over the commit hash.

However, the author date and commit date are measure in a resolution of one
second, and who really cares if you committed your code at 4:38:07 pm or
4:41:19 pm? Nobody, that's who. 

This program exploits that fact and twiddles with the author and committer date
until it finds some that make the commit hash match the desired prefix.


## To use

Build the program using `cargo build --release`.

Then copy `target/release/git-force-prefix` to somewhere in your `$PATH`.

The next time you want to force a commit hash prefix, first make the commit,
then run `git force-prefix 012345` to get a commit with the first 6 characters
of the commit hash as 012345. The output will be a command that can be run to
change the commit hash.


## Dogfooding

Every commit in this repository starts with the prefix `d06f00d` because this
project dogfood's its own process.
