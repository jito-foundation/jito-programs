use std::collections::BTreeMap;

use anchor_lang::{
    error::ErrorCode::{AccountDidNotDeserialize, ConstraintOwner},
    prelude::{AccountInfo, Pubkey, Result},
};
use bincode::deserialize;
use serde_derive::Deserialize;

pub struct VoteState;

impl VoteState {
    pub fn deserialize_node_pubkey(account_info: &AccountInfo) -> Result<Pubkey> {
        if Pubkey::from(account_info.owner.to_bytes())
            != Pubkey::from(solana_sdk_ids::vote::id().to_bytes())
        {
            return Err(ConstraintOwner.into());
        }

        // The first 4 bytes are the enumeration type and the next 32 bytes of the vote state are the node pubkey.
        let data = account_info.data.borrow();
        deserialize::<Pubkey>(&data[4..36]).map_err(|_| AccountDidNotDeserialize.into())
    }

    /// Returns the vote account's authorized voters, keyed by the epoch each takes effect.
    /// Empty for versions before V4: the vote program rewrites an account as V4 whenever its
    /// identity or voter changes, so an account mid-rotation is always V4.
    pub fn deserialize_authorized_voters(
        account_info: &AccountInfo,
    ) -> Result<BTreeMap<u64, Pubkey>> {
        Self::deserialize_node_pubkey(account_info)?; // checks the vote program owns the account

        match deserialize(&account_info.data.borrow()) {
            Ok(VoteStateVersions::V4(vote_state)) => Ok(vote_state.authorized_voters),
            Ok(_) => Ok(BTreeMap::new()),
            Err(_) => Err(AccountDidNotDeserialize.into()),
        }
    }
}

/// solana-vote-interface's `VoteStateVersions`. Only V4 is read, and only up to
/// `authorized_voters`; bincode ignores the remaining bytes.
#[derive(Deserialize)]
enum VoteStateVersions {
    Uninitialized,
    V1_14_11,
    V3,
    V4(VoteStateV4),
}

#[allow(dead_code)] // fields before `authorized_voters` are read only to skip them
#[derive(Deserialize)]
struct VoteStateV4 {
    node_pubkey: Pubkey,
    authorized_withdrawer: Pubkey,
    inflation_rewards_collector: Pubkey,
    block_revenue_collector: Pubkey,
    inflation_rewards_commission_bps: u16,
    block_revenue_commission_bps: u16,
    pending_delegator_rewards: u64,
    bls_pubkey_compressed: Option<([u8; 32], [u8; 16])>, // [u8; 48]
    votes: Vec<(u8, u64, u32)>, // LandedVote { latency, lockout: { slot, confirmation_count } }
    root_slot: Option<u64>,
    authorized_voters: BTreeMap<u64, Pubkey>,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    /// A V4 vote account mid identity rotation, serialized by solana-vote-interface 7.3.0:
    /// node = new identity, authorized voters {700: old, 702: new}.
    const ROTATION: &[u8] = include_bytes!("../tests/fixtures/vote-account-rotation.bin");

    fn voters(data: &[u8], owner: &Pubkey) -> Result<BTreeMap<u64, Pubkey>> {
        let (key, mut lamports, mut data) = (Pubkey::new_unique(), 1, data.to_vec());
        let account = AccountInfo::new(
            &key,
            false,
            false,
            &mut lamports,
            &mut data,
            owner,
            false,
            0,
        );
        VoteState::deserialize_authorized_voters(&account)
    }

    #[test]
    fn test_deserialize_authorized_voters() {
        let vote_program = Pubkey::from(solana_sdk_ids::vote::id().to_bytes());
        let old = Pubkey::from_str("AKnL4NNf3DGWZJS6cPknBuEGnVsV4A4m5tgebLHaRSZ9").unwrap();
        let new = Pubkey::from_str("9hSR6S7WPtxmTojgo6GG3k4yDPecgJY292j7xrsUGWBu").unwrap();
        assert_eq!(
            voters(ROTATION, &vote_program).unwrap(),
            BTreeMap::from([(700, old), (702, new)])
        );

        let mut v3 = ROTATION.to_vec();
        v3[..4].copy_from_slice(&2u32.to_le_bytes());
        assert!(voters(&v3, &vote_program).unwrap().is_empty());

        assert!(voters(ROTATION, &Pubkey::new_unique()).is_err());
    }
}
