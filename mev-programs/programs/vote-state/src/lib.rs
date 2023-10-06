#![allow(clippy::all)] // ignore this file so cargo clippy in jito-solana repo passes
//! This code was mostly copy-pasta'd from [here](https://github.com/solana-labs/solana/blob/df128573127c324cb5b53634a7e2d77427c6f2d8/programs/vote/src/vote_state/mod.rs#L1).
//! In all current releases [VoteState] is defined in the `solana-vote-program` crate which is not compatible
//! with programs targeting BPF bytecode due to some BPF-incompatible libraries being pulled in.

use std::collections::{BTreeMap, VecDeque};

use anchor_lang::{
    error::ErrorCode::{AccountDidNotDeserialize, ConstraintOwner},
    prelude::*,
};
use bincode::deserialize;
use serde_derive::Deserialize;

type Epoch = u64;
type Slot = u64;
type UnixTimestamp = i64;

#[derive(Clone, Deserialize)]
pub struct Lockout {
    pub slot: Slot,
    pub confirmation_count: u32,
}

#[derive(Default, Deserialize)]
struct AuthorizedVoters {
    authorized_voters: BTreeMap<Epoch, Pubkey>,
}

impl AuthorizedVoters {
    pub fn new(epoch: Epoch, pubkey: Pubkey) -> Self {
        let mut authorized_voters = BTreeMap::new();
        authorized_voters.insert(epoch, pubkey);
        Self { authorized_voters }
    }
}

const MAX_ITEMS: usize = 32;

#[derive(Default, Deserialize)]
pub struct CircBuf<I> {
    buf: [I; MAX_ITEMS],
    /// next pointer
    idx: usize,
    is_empty: bool,
}

#[derive(Clone, Deserialize, Default)]
pub struct BlockTimestamp {
    pub slot: Slot,
    pub timestamp: UnixTimestamp,
}

#[derive(Deserialize)]
pub enum VoteStateVersions {
    V0_23_5(Box<VoteState0_23_5>),
    Current(Box<VoteState>),
}

impl VoteStateVersions {
    pub fn convert_to_current(self) -> Box<VoteState> {
        match self {
            VoteStateVersions::V0_23_5(state) => {
                let authorized_voters =
                    AuthorizedVoters::new(state.authorized_voter_epoch, state.authorized_voter);

                Box::new(VoteState {
                    node_pubkey: state.node_pubkey,

                    /// the signer for withdrawals
                    authorized_withdrawer: state.authorized_withdrawer,

                    /// percentage (0-100) that represents what part of a rewards
                    ///  payout should be given to this VoteAccount
                    commission: state.commission,

                    votes: state.votes,

                    root_slot: state.root_slot,
                    /// the signer for vote transactions
                    authorized_voters,
                    /// history of prior authorized voters and the epochs for which
                    /// they were set, the bottom end of the range is inclusive,
                    /// the top of the range is exclusive
                    prior_voters: CircBuf::default(),

                    /// history of how many credits earned by the end of each epoch
                    ///  each tuple is (Epoch, credits, prev_credits)
                    epoch_credits: state.epoch_credits,

                    /// most recent timestamp submitted with a vote
                    last_timestamp: state.last_timestamp,
                })
            }
            VoteStateVersions::Current(state) => state,
        }
    }
}

#[derive(Deserialize)]
pub struct VoteState {
    /// the node that votes in this account
    pub node_pubkey: Pubkey,

    /// the signer for withdrawals
    #[serde(skip_deserializing)]
    pub authorized_withdrawer: Pubkey,
    /// percentage (0-100) that represents what part of a rewards
    ///  payout should be given to this VoteAccount
    #[serde(skip_deserializing)]
    pub commission: u8,
    #[serde(skip_deserializing)]
    pub votes: VecDeque<Lockout>,

    /// This usually the last Lockout which was popped from self.votes.
    /// However, it can be arbitrary slot, when being used inside Tower
    #[serde(skip_deserializing)]
    pub root_slot: Option<Slot>,

    /// the signer for vote transactions
    #[serde(skip_deserializing)]
    authorized_voters: AuthorizedVoters,

    /// history of prior authorized voters and the epochs for which
    /// they were set, the bottom end of the range is inclusive,
    /// the top of the range is exclusive
    #[serde(skip_deserializing)]
    prior_voters: CircBuf<(Pubkey, Epoch, Epoch)>,

    /// history of how many credits earned by the end of each epoch
    ///  each tuple is (Epoch, credits, prev_credits)
    #[serde(skip_deserializing)]
    pub(crate) epoch_credits: Vec<(Epoch, u64, u64)>,

    /// most recent timestamp submitted with a vote
    #[serde(skip_deserializing)]
    pub last_timestamp: BlockTimestamp,
}

#[derive(Deserialize)]
pub struct VoteState0_23_5 {
    /// the node that votes in this account
    pub node_pubkey: Pubkey,

    /// the signer for vote transactions
    #[serde(skip_deserializing)]
    pub authorized_voter: Pubkey,
    /// when the authorized voter was set/initialized
    #[serde(skip_deserializing)]
    pub authorized_voter_epoch: Epoch,

    /// history of prior authorized voters and the epoch ranges for which
    ///  they were set
    #[serde(skip_deserializing)]
    pub prior_voters: CircBuf<(Pubkey, Epoch, Epoch, Slot)>,

    /// the signer for withdrawals
    #[serde(skip_deserializing)]
    pub authorized_withdrawer: Pubkey,
    /// percentage (0-100) that represents what part of a rewards
    /// payout should be given to this VoteAccount
    #[serde(skip_deserializing)]
    pub commission: u8,

    #[serde(skip_deserializing)]
    pub votes: VecDeque<Lockout>,
    #[serde(skip_deserializing)]
    pub root_slot: Option<u64>,

    /// history of how many credits earned by the end of each epoch
    ///  each tuple is (Epoch, credits, prev_credits)
    #[serde(skip_deserializing)]
    pub epoch_credits: Vec<(Epoch, u64, u64)>,

    /// most recent timestamp submitted with a vote
    #[serde(skip_deserializing)]
    pub last_timestamp: BlockTimestamp,
}

impl VoteState {
    pub fn deserialize(account_info: &AccountInfo) -> Result<Box<Self>> {
        if account_info.owner != &solana_program::vote::program::id() {
            return Err(ConstraintOwner.into());
        }

        let data = account_info.data.borrow();
        deserialize::<Box<VoteStateVersions>>(&data)
            .map(|v| v.convert_to_current())
            .map_err(|_| AccountDidNotDeserialize.into())
    }
}
