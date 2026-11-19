# CharSheet

## Releasing

Releases go through a PR; nothing is tagged by hand.

1. `make changes` lists the commits since the last release tag. Read them and
   write a short Markdown summary for users (highlights first, no commit
   hashes) to a file in your scratchpad.
2. `make release VERSION=x.y.z NOTES=/path/to/notes.md` bumps the version in
   Cargo.toml, Cargo.lock and the metainfo on a `release/vx.y.z` branch,
   pushes it and opens a PR. Run it from a clean, pushed master.
3. When that PR is merged, `.github/workflows/release.yml` tags the merge
   commit and publishes the GitHub release: the summary, then a changelog
   generated from git history, then GitHub's generated notes.
4. `make release-notes VERSION=x.y.z NOTES=/path/to/notes.md` replaces the
   summary, on the open PR before merging or on the published release after.
