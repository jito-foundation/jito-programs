# Jito Programs
This repository is home to Jito's on-chain programs that enable MEV collection and MEV sharing with SOL stakers; additionally
we may host useful on-chain program reference implementations here.

## Gitflow
This repository is declared as a submodule in the `jito-solana` client. In order for that to work the `Solana`
crates declared in this repo must point to the `jito-solana` declaring this as a submodule. The `Anchor` dependencies
must also point to the local path where `anchor` resides as a submodule within `jito-solana`. This is due to dependency
version issues. With that said, `jito-solana` declares a dependency on the `submodule` branch found in this repository.
This gitflow for this repo is as follows:
- Create a PR against `master`
- Merge PR
- `git cherry-pick` your new commits into `submodule`
- push `submodule`
- 

## API

### Run

```bash
cd api

cargo r -- --bind-addr <BIND_ADDR>  --rpc-url <RPC_URL> --jito-tip-distribution-program-id <JITO_TIP_DISTRIBUTION_PROGRAM_ID>
```

### Send a request

```bash
curl http://localhost:7001/get_tip_distribution/{vote_account}/{epoch} 
```
