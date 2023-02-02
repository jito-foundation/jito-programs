# jito-programs
This repository holds a set of programs necessary for MEV revenue sharing.

## tip-payment
This program owns the PDAs and contains the instructions necessary to enable validator tips. 
There are multiple PDAs that searchers may tip, this is to enable a greater degree of concurrency.
The validator workflow is as follows for every slot:
- Searchers submit bundles and include an instruction or transaction that funds one of the many tip payment PDAs
(note that these PDAs should not be mixed up with the tip distribution account PDA).
- Upon receiving the first bundle for the slot, the validator fetches the current configured tip receiver set on this program.
- If the tip receiver is not equal to the validator's tip distribution account PDA (owned by the tip-distribution program),
then invoke the `change_tip_receiver` instruction, supplying the validator's tip distribution account PDA.
- The `change_tip_receiver` instruction transfers the tips out of all the tip payment PDAs to the previously configured receiver
before setting the new validator as the receiver.

## tip-distribution
This program is responsible for distributing MEV to the rest of the network and functions similarly airdrops, leveraging
the use of merkle trees. Workflow:
- Every epoch validators initialize a tip distribution account. These PDAs are derived from the validator's vote account and given epoch.
Another way to think about these accounts is in terms of MEV buckets scoped per validator per epoch. All lamports in these accounts
are considered MEV earned by the PDAs corresponding validator for the epoch.
- Upon initialization of their respective accounts, validators specify an authority that has the power to generate a merkle root and upload it.
The merkle roots are what is used to determine what portion of the MEV in the bucket stakers are entitled to.
- Once the epoch comes to a close the merkle root authority is able to generate a merkle root and upload it.
[Here](https://github.com/jito-foundation/jito-solana/tree/master/tip-distributor) is an example of what that workflow could look like.
- Claimants/Stakers then have up to some configured number of epochs to claim their share of the MEV across all buckets.

## Deployments
The `master` branch may not always be what's deployed on-chain, instead the `deployed/tip-payment` and `deployed/tip-distribution`
branches designate what's actually deployed on-chain for the individual programs.

## Gitflow
All PRs shall be made against master. Upon merge make sure to cherry-pick the new commits to the `submodule` branch.
The `jito-solana` client declares this branch as a submodule dependency in order to interact with the two programs.
Once code is deployed make sure to cherry-pick the appropriate commits to the programs' repsective `deployed/*` branches.
