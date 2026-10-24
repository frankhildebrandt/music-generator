# Repository history and fresh checkouts

On 24 October 2026, the fork's Git history was rewritten to use `de.alphatester` consistently in package identifiers, service paths and documentation. All five development branches were included. Historical migration guidance now describes selecting the previous profile without naming another package identifier.

Authorship, timestamps, merge structure, file paths and file modes were preserved. An exhaustive comparison checked 15,519 file versions against the intended replacements and documentation adjustment. A fresh normal clone had zero occurrences of the former identifier anywhere in its reachable Git objects, including commit messages. Git object integrity checks passed. The original repository was saved separately in a local Git bundle before publication.

The nine test releases from 0.2.0 through 0.2.8, their 101 downloadable assets and their test tags were subsequently removed from the fork. Their release descriptions, asset metadata and previous Git references were backed up locally. No historical binaries were rebuilt or relabeled. Current development code and existing user libraries, models and song files were retained.

## Continue with a fresh clone

The rewritten commits have new IDs. Existing clones and worktrees still contain the previous history; merging or force-pushing those old branches can reintroduce it. Continue from a fresh clone of `frankhildebrandt/music-generator` and check out the required development branch. Transfer any uncommitted work as a reviewed patch onto that clean history, rather than merging the old lineage.

Use a separate clean checkout on the required development branch. Previous local working copies were deliberately left unchanged. The build workflows were restored to their original enabled state after publication.

## GitHub-retained references

GitHub manages pull-request refs, old CI records and cached commit pages separately from normal branches and tags. A force-push cannot replace the immutable head of an already merged pull request; fork PR #3 still retains its historical head. Old references in the original repository are also outside this fork's writable Git refs. This cleanup therefore guarantees the normal branch/tag history and fresh clones, rather than deletion of all copies cached by GitHub or other clones.

For a fully independent repository, publish only the cleaned branches to a new repository outside the existing fork network. Do not mirror the old pull-request refs or old local branches into that destination.
