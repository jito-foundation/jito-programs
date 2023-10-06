#![allow(dead_code)]
//! This code was mostly copy-pasta'd from [here](https://github.com/solana-labs/solana/blob/df128573127c324cb5b53634a7e2d77427c6f2d8/programs/vote/src/vote_state/mod.rs#L1).
//! In all current releases [VoteState1_14_11] is defined in the `solana-vote-program` crate which is not compatible
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

#[derive(Clone, Default, Deserialize)]
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
    V1_14_11(Box<VoteState1_14_11>),
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

                    authorized_withdrawer: state.authorized_withdrawer,

                    commission: state.commission,

                    votes: Self::landed_votes_from_lockouts(state.votes),

                    root_slot: state.root_slot,

                    authorized_voters,

                    prior_voters: CircBuf::default(),

                    epoch_credits: state.epoch_credits.clone(),

                    last_timestamp: state.last_timestamp.clone(),
                })
            }
            VoteStateVersions::V1_14_11(state) => Box::new(VoteState {
                node_pubkey: state.node_pubkey,
                authorized_withdrawer: state.authorized_withdrawer,
                commission: state.commission,

                votes: Self::landed_votes_from_lockouts(state.votes),

                root_slot: state.root_slot,

                authorized_voters: state.authorized_voters.clone(),

                prior_voters: state.prior_voters,

                epoch_credits: state.epoch_credits,

                last_timestamp: state.last_timestamp,
            }),
            VoteStateVersions::Current(state) => state,
        }
    }

    fn landed_votes_from_lockouts(lockouts: VecDeque<Lockout>) -> VecDeque<LandedVote> {
        lockouts.into_iter().map(|lockout| lockout.into()).collect()
    }
}

#[derive(Deserialize)]
pub struct VoteState1_14_11 {
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
    pub votes: VecDeque<LandedVote>,

    // This usually the last Lockout which was popped from self.votes.
    // However, it can be arbitrary slot, when being used inside Tower
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
    pub epoch_credits: Vec<(Epoch, u64, u64)>,

    /// most recent timestamp submitted with a vote
    #[serde(skip_deserializing)]
    pub last_timestamp: BlockTimestamp,
}

#[derive(Deserialize)]
pub struct LandedVote {
    // Latency is the difference in slot number between the slot that was voted on (lockout.slot) and the slot in
    // which the vote that added this Lockout landed.  For votes which were cast before versions of the validator
    // software which recorded vote latencies, latency is recorded as 0.
    pub latency: u8,
    pub lockout: Lockout,
}

impl LandedVote {
    pub fn slot(&self) -> Slot {
        self.lockout.slot
    }

    pub fn confirmation_count(&self) -> u32 {
        self.lockout.confirmation_count
    }
}

impl From<LandedVote> for Lockout {
    fn from(landed_vote: LandedVote) -> Self {
        landed_vote.lockout
    }
}

impl From<Lockout> for LandedVote {
    fn from(lockout: Lockout) -> Self {
        Self {
            latency: 0,
            lockout,
        }
    }
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
