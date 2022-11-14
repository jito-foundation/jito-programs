# Jito Programs
This repository is home to our on-chain programs that enable MEV collection and MEV sharing with SOL stakers.

## Gitflow
This repository is declared as a submodule in the `jito-solana` client. In order for that to work the `Solana`
crates declared in this repo must point to the `jito-solana` declaring this as a submodule. The `Anchor` dependencies
must also point to the local path where `anchor` resides as a submodule within `jito-solana`. This is due to dependency
version issues. With that said, `jito-solana` declares a dependency on the `submodule` branch found in this repository.
This gitflow for this repo is as follows:
- Create a PR against `master`
- Merge PR
- Rebase `submodule` on top of master
- Force push `submodule`
- Update the submodule sha in `jito-solana`
