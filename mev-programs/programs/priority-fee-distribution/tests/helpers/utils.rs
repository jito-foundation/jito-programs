use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use solana_program_test::ProgramTestContext;
use solana_sdk::{
    account::Account,
    clock::Epoch,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction, system_program,
    transaction::Transaction,
    vote::{
        instruction::{self as vote_instruction, CreateVoteAccountConfig},
        state::{VoteInit, VoteState},
    },
};

use jito_priority_fee_distribution::{
    accounts, instruction,
    state::{Config, MerkleRoot, PriorityFeeDistributionAccount},
};

use crate::{helpers::merkle_tree::MerkleTree, CLAIM_STATUS_SEED, ROOT_UPLOAD_CONFIG_SEED};

pub async fn sleep_for_epochs(ctx: &mut ProgramTestContext, num_epochs: u64) {
    let clock = ctx
        .banks_client
        .get_sysvar::<solana_sdk::sysvar::clock::Clock>()
        .await
        .unwrap();

    ctx.warp_to_epoch(clock.epoch + num_epochs).unwrap();
}

pub fn balance_to_leaf_bytes(account: &Pubkey, amount: u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(40);
    out.extend_from_slice(account.as_ref());
    out.extend_from_slice(&amount.to_le_bytes());
    out
}

pub fn assert_config_state(actual: &Config, expected: &Config) {
    assert_eq!(actual.authority, expected.authority);

    assert_eq!(actual.expired_funds_account, expected.expired_funds_account);

    assert_eq!(
        actual.max_validator_commission_bps,
        expected.max_validator_commission_bps
    );

    assert_eq!(actual.num_epochs_valid, expected.num_epochs_valid);
}

pub fn assert_distribution_account(
    actual: &PriorityFeeDistributionAccount,
    expected: &PriorityFeeDistributionAccount,
) {
    assert_eq!(
        actual.validator_vote_account,
        expected.validator_vote_account
    );

    assert_eq!(
        actual.merkle_root_upload_authority,
        expected.merkle_root_upload_authority
    );

    assert_eq!(actual.epoch_created_at, expected.epoch_created_at);

    assert_eq!(
        actual.validator_commission_bps,
        expected.validator_commission_bps
    );

    match (&actual.merkle_root, &expected.merkle_root) {
        (Some(a), Some(e)) => {
            assert_merkle_root(a, e);
        }
        (None, None) => {}
        _ => panic!("Merkle root mismatch: one is Some, the other is None"),
    }
}

fn assert_merkle_root(actual: &MerkleRoot, expected: &MerkleRoot) {
    assert_eq!(actual.root, expected.root);
    assert_eq!(actual.max_total_claim, expected.max_total_claim);
    assert_eq!(actual.max_num_nodes, expected.max_num_nodes);
    assert_eq!(actual.total_funds_claimed, expected.total_funds_claimed);
    assert_eq!(actual.num_nodes_claimed, expected.num_nodes_claimed);
}

pub async fn generate_account(ctx: &ProgramTestContext, airdrop_amount: u64) -> Keypair {
    let account = Keypair::new();

    if airdrop_amount > 0 {
        let ix =
            system_instruction::transfer(&ctx.payer.pubkey(), &account.pubkey(), airdrop_amount);

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer],
            ctx.last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await.unwrap();
    }

    account
}

pub fn derive_priority_fee_distribution_account_address(
    vote_pubkey: &Pubkey,
    epoch: u64,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            PriorityFeeDistributionAccount::SEED,
            vote_pubkey.as_ref(),
            &epoch.to_le_bytes(),
        ],
        &jito_priority_fee_distribution::id(),
    )
}

pub fn derive_priority_fee_distribution_account_address_with_id(
    program_id: &Pubkey,
    vote_pubkey: &Pubkey,
    epoch: u64,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            PriorityFeeDistributionAccount::SEED,
            vote_pubkey.as_ref(),
            &epoch.to_le_bytes(),
        ],
        program_id,
    )
}

pub fn derive_config_account_address() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[Config::SEED], &jito_priority_fee_distribution::id())
}

pub fn derive_config_account_address_with_id(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[Config::SEED], program_id)
}

pub fn derive_claim_status_account_address(
    claimant: &Pubkey,
    priority_fee_distribution_account: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            CLAIM_STATUS_SEED.as_bytes(),
            claimant.as_ref(),
            priority_fee_distribution_account.as_ref(),
        ],
        &jito_priority_fee_distribution::id(),
    )
}

pub fn derive_merkle_root_upload_config_address() -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[ROOT_UPLOAD_CONFIG_SEED.as_bytes()],
        &jito_priority_fee_distribution::id(),
    )
}

pub struct InitTipDistributionSetup {
    pub max_validator_commission_bps: u16,
    pub validator_identity: Keypair,
    pub validator_vote_account: Keypair,
    pub priority_fee_distribution_account: Pubkey,
    pub bump: u8,
    pub epoch: u64,
}

pub async fn setup_init_tip_distribution_account(
    ctx: &ProgramTestContext,
    program_id: &Pubkey,
) -> InitTipDistributionSetup {
    let (config_pda, _) = derive_config_account_address_with_id(program_id);

    let config_account = ctx
        .banks_client
        .get_account(config_pda)
        .await
        .unwrap()
        .unwrap();

    let config: Config = Config::try_deserialize(&mut &config_account.data[..]).unwrap();

    let validator_identity = generate_account(ctx, 10_000_000_000).await;

    let validator_vote_account = Keypair::new();

    let vote_init = VoteInit {
        node_pubkey: validator_identity.pubkey(),
        authorized_voter: validator_identity.pubkey(),
        authorized_withdrawer: validator_identity.pubkey(),
        commission: 0,
    };

    let rent = ctx.banks_client.get_rent().await.unwrap();
    let lamports = rent.minimum_balance(VoteState::size_of())
        + 10 * solana_sdk::native_token::LAMPORTS_PER_SOL;

    let create_vote_ix = vote_instruction::create_account_with_config(
        &ctx.payer.pubkey(),
        &validator_vote_account.pubkey(),
        &vote_init,
        lamports,
        CreateVoteAccountConfig {
            space: VoteState::size_of() as u64,
            with_seed: None,
        },
    );

    let tx = Transaction::new_signed_with_payer(
        &create_vote_ix,
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &validator_vote_account, &validator_identity],
        ctx.last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await.unwrap();

    let clock = ctx
        .banks_client
        .get_sysvar::<solana_sdk::sysvar::clock::Clock>()
        .await
        .unwrap();
    let epoch = clock.epoch;

    let (priority_fee_distribution_account, bump) =
        derive_priority_fee_distribution_account_address_with_id(
            &program_id,
            &validator_vote_account.pubkey(),
            epoch,
        );

    InitTipDistributionSetup {
        max_validator_commission_bps: config.max_validator_commission_bps,
        validator_identity,
        validator_vote_account,
        priority_fee_distribution_account,
        bump,
        epoch,
    }
}

pub async fn call_initialize(
    ctx: &ProgramTestContext,
    authority: &Keypair,
    expired_funds_account: &Pubkey,
    num_epochs_valid: u64,
    max_validator_commission_bps: u16,
    config_bump: u8,
    config_account: &Pubkey,
    initializer: &Keypair,
) -> Result<(), solana_program_test::BanksClientError> {
    let ix_data = instruction::Initialize {
        authority: authority.pubkey(),
        expired_funds_account: *expired_funds_account,
        num_epochs_valid,
        max_validator_commission_bps,
        bump: config_bump,
    };

    let accounts = accounts::Initialize {
        config: *config_account,
        system_program: system_program::id(),
        initializer: initializer.pubkey(),
    };

    let ix = solana_sdk::instruction::Instruction {
        program_id: jito_priority_fee_distribution::id(),
        accounts: accounts.to_account_metas(None),
        data: ix_data.data(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, initializer],
        ctx.last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await
}

pub async fn call_init_tip_distribution_account(
    ctx: &ProgramTestContext,
    program_id: &Pubkey,
    validator_commission_bps: u16,
    merkle_root_upload_authority: &Pubkey,
    config: &Pubkey,
    validator_identity: &Keypair,
    validator_vote_account: &Pubkey,
    priority_fee_distribution_account: &Pubkey,
    bump: u8,
) -> Result<(), solana_program_test::BanksClientError> {
    let ix = instruction::InitializePriorityFeeDistributionAccount {
        merkle_root_upload_authority: *merkle_root_upload_authority,
        validator_commission_bps,
        bump,
    };

    let accounts = accounts::InitializePriorityFeeDistributionAccount {
        config: *config,
        system_program: system_program::id(),
        signer: validator_identity.pubkey(),
        validator_vote_account: *validator_vote_account,
        priority_fee_distribution_account: *priority_fee_distribution_account,
    };

    let instruction = solana_sdk::instruction::Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: ix.data(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, validator_identity],
        ctx.last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await
}

pub async fn call_close_priority_fee_distribution_account(
    ctx: &ProgramTestContext,
    config: &Pubkey,
    expired_funds_account: &Pubkey,
    priority_fee_distribution_account: &Pubkey,
    validator_vote_account: &Pubkey,
    epoch: u64,
) -> Result<(), solana_program_test::BanksClientError> {
    let ix_data = instruction::ClosePriorityFeeDistributionAccount { _epoch: epoch };

    let accounts = accounts::ClosePriorityFeeDistributionAccount {
        config: *config,
        expired_funds_account: *expired_funds_account,
        priority_fee_distribution_account: *priority_fee_distribution_account,
        validator_vote_account: *validator_vote_account,
        signer: ctx.payer.pubkey(),
    };

    let ix = solana_sdk::instruction::Instruction {
        program_id: jito_priority_fee_distribution::id(),
        accounts: accounts.to_account_metas(None),
        data: ix_data.data(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await
}

pub async fn call_claim(
    ctx: &ProgramTestContext,
    priority_fee_distribution_account: &Pubkey,
    validator_vote_account: &Keypair,
    claim_status: &Pubkey,
    claimant: &Keypair,
    payer: &Keypair,
    amount: u64,
    proof: Vec<[u8; 32]>,
) -> Result<(), solana_program_test::BanksClientError> {
    let (_claim_status_address, bump) =
        derive_claim_status_account_address(&claimant.pubkey(), priority_fee_distribution_account);

    let ix_data = instruction::Claim {
        _bump: bump,
        amount,
        proof,
    };

    let accounts = accounts::Claim {
        config: derive_config_account_address().0,
        priority_fee_distribution_account: *priority_fee_distribution_account,
        merkle_root_upload_authority: validator_vote_account.pubkey(),
        claim_status: *claim_status,
        claimant: claimant.pubkey(),
        payer: payer.pubkey(),
        system_program: system_program::id(),
    };

    let ix = solana_sdk::instruction::Instruction {
        program_id: jito_priority_fee_distribution::id(),
        accounts: accounts.to_account_metas(None),
        data: ix_data.data(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, payer, validator_vote_account],
        ctx.last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await
}

pub async fn call_close_claim_status(
    ctx: &ProgramTestContext,
    claim_status: &Pubkey,
    claim_status_payer: &Keypair,
) -> Result<(), solana_program_test::BanksClientError> {
    let ix_data = instruction::CloseClaimStatus {};

    let accounts = accounts::CloseClaimStatus {
        claim_status: *claim_status,
        claim_status_payer: claim_status_payer.pubkey(),
    };

    let ix = solana_sdk::instruction::Instruction {
        program_id: jito_priority_fee_distribution::id(),
        accounts: accounts.to_account_metas(None),
        data: ix_data.data(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await
}

pub struct MerkleSetup {
    pub amount0: u64,
    pub amount1: u64,
    pub pre_balance0: u64,
    pub root: [u8; 32],
    pub priority_fee_distribution_account: Pubkey,
    pub tree: MerkleTree,
    pub user0: Keypair,
    pub user1: Keypair,
    pub validator_vote_account: Keypair,
    pub epoch: Epoch,
    pub bump: u8,
    pub max_validator_commission_bps: u16,
    pub max_total_claim: u64,
    pub max_num_nodes: u64,
}

pub async fn setup_with_uploaded_merkle_root(
    ctx: &mut ProgramTestContext,
    program_id: &Pubkey,
) -> MerkleSetup {
    let setup = setup_init_tip_distribution_account(ctx, &program_id).await;

    call_init_tip_distribution_account(
        ctx,
        program_id,
        setup.max_validator_commission_bps,
        &setup.validator_vote_account.pubkey(),
        &derive_config_account_address_with_id(&program_id).0,
        &setup.validator_identity,
        &setup.validator_vote_account.pubkey(),
        &setup.priority_fee_distribution_account,
        setup.bump,
    )
    .await
    .unwrap();

    let amount0 = 1_000_000;
    let amount1 = 2_000_000;

    let fund_tx = solana_sdk::system_instruction::transfer(
        &ctx.payer.pubkey(),
        &setup.priority_fee_distribution_account,
        amount0 + amount1,
    );

    let tx = Transaction::new_signed_with_payer(
        &[fund_tx],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await.unwrap();

    let pre_balance0 = 10_000_000_000;
    let user0 = generate_account(ctx, pre_balance0).await;
    let user1 = generate_account(ctx, pre_balance0).await;

    let demo_data = vec![
        balance_to_leaf_bytes(&user0.pubkey(), amount0),
        balance_to_leaf_bytes(&user1.pubkey(), amount1),
    ];

    let tree = MerkleTree::new(demo_data);
    let root = tree.get_root();

    let max_total_claim = amount0 + amount1;
    let max_num_nodes = 2u64;

    sleep_for_epochs(ctx, 1).await;

    let ix = instruction::UploadMerkleRoot {
        root,
        max_total_claim,
        max_num_nodes,
    };

    let accounts = accounts::UploadMerkleRoot {
        priority_fee_distribution_account: setup.priority_fee_distribution_account,
        merkle_root_upload_authority: setup.validator_vote_account.pubkey(),
        config: derive_config_account_address_with_id(&program_id).0,
    };

    let instruction = solana_sdk::instruction::Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: ix.data(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &setup.validator_vote_account],
        ctx.last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await.unwrap();

    MerkleSetup {
        amount0,
        amount1,
        pre_balance0,
        root,
        priority_fee_distribution_account: setup.priority_fee_distribution_account,
        tree,
        user0,
        user1,
        validator_vote_account: setup.validator_vote_account,
        bump: setup.bump,
        epoch: setup.epoch,
        max_validator_commission_bps: setup.max_validator_commission_bps,
        max_num_nodes,
        max_total_claim,
    }
}

pub async fn get_deserialized_account<T: anchor_lang::AccountDeserialize>(
    ctx: &ProgramTestContext,
    key: &Pubkey,
) -> Result<Option<T>, anchor_lang::error::Error> {
    let acc_opt = ctx.banks_client.get_account(*key).await.unwrap_or_default();

    match acc_opt {
        None => Ok(None),
        Some(acc) => {
            let mut data: &[u8] = &acc.data[..];
            let deserialized = T::try_deserialize(&mut data)?;
            Ok(Some(deserialized))
        }
    }
}

pub async fn get_account(
    ctx: &ProgramTestContext,
    key: &Pubkey,
) -> Result<Option<Account>, solana_program_test::BanksClientError> {
    ctx.banks_client.get_account(*key).await
}

pub async fn get_balance(
    ctx: &ProgramTestContext,
    key: &Pubkey,
) -> Result<u64, solana_program_test::BanksClientError> {
    ctx.banks_client.get_balance(*key).await
}

pub async fn call_upload_merkle_root(
    ctx: &ProgramTestContext,
    priority_fee_distribution_account: &Pubkey,
    merkle_root_upload_authority: &Keypair,
    root: [u8; 32],
    max_total_claim: u64,
    max_num_nodes: u64,
) -> Result<(), solana_program_test::BanksClientError> {
    let ix_data = instruction::UploadMerkleRoot {
        root,
        max_total_claim,
        max_num_nodes,
    };

    let accounts = accounts::UploadMerkleRoot {
        priority_fee_distribution_account: *priority_fee_distribution_account,
        merkle_root_upload_authority: merkle_root_upload_authority.pubkey(),
        config: derive_config_account_address().0,
    };

    let ix = solana_sdk::instruction::Instruction {
        program_id: jito_priority_fee_distribution::id(),
        accounts: accounts.to_account_metas(None),
        data: ix_data.data(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, merkle_root_upload_authority],
        ctx.last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await
}

pub async fn call_initialize_merkle_root_upload_config(
    ctx: &ProgramTestContext,
    config_account: &Pubkey,
    merkle_root_upload_config_key: &Pubkey,
    authority: &Keypair,
    override_authority: &Keypair,
    original_authority: &Pubkey,
) -> Result<(), solana_program_test::BanksClientError> {
    let ix_data = instruction::InitializeMerkleRootUploadConfig {
        authority: override_authority.pubkey(),
        original_authority: *original_authority,
    };

    let accounts = accounts::InitializeMerkleRootUploadConfig {
        config: *config_account,
        merkle_root_upload_config: *merkle_root_upload_config_key,
        authority: authority.pubkey(),
        payer: ctx.payer.pubkey(),
        system_program: system_program::id(),
    };

    let ix = solana_sdk::instruction::Instruction {
        program_id: jito_priority_fee_distribution::id(),
        accounts: accounts.to_account_metas(None),
        data: ix_data.data(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, authority],
        ctx.last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await
}

pub async fn call_update_merkle_root_upload_config(
    ctx: &ProgramTestContext,
    config_account: &Pubkey,
    merkle_root_upload_config_key: &Pubkey,
    authority: &Keypair,
    new_override_authority: &Pubkey,
    original_authority: &Pubkey,
) -> Result<(), solana_program_test::BanksClientError> {
    let ix_data = instruction::UpdateMerkleRootUploadConfig {
        authority: *new_override_authority,
        original_authority: *original_authority,
    };

    let accounts = accounts::UpdateMerkleRootUploadConfig {
        config: *config_account,
        merkle_root_upload_config: *merkle_root_upload_config_key,
        authority: authority.pubkey(),
        system_program: system_program::id(),
    };

    let ix = solana_sdk::instruction::Instruction {
        program_id: jito_priority_fee_distribution::id(),
        accounts: accounts.to_account_metas(None),
        data: ix_data.data(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, authority],
        ctx.last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await
}

pub async fn call_update_config(
    ctx: &ProgramTestContext,
    new_config: Config,
    authority: &Keypair,
    config_pda: &Pubkey,
) -> Result<(), solana_program_test::BanksClientError> {
    let ix_data = instruction::UpdateConfig { new_config };

    let accounts = accounts::UpdateConfig {
        config: *config_pda,
        authority: authority.pubkey(),
    };

    let ix = solana_sdk::instruction::Instruction {
        program_id: jito_priority_fee_distribution::id(),
        accounts: accounts.to_account_metas(None),
        data: ix_data.data(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, authority],
        ctx.last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await
}

pub async fn call_transfer_priority_fee_tips(
    ctx: &ProgramTestContext,
    program_id: &Pubkey,
    config: &Pubkey,
    priority_fee_distribution_account: &Pubkey,
    from: &Keypair,
    lamports: u64,
) -> Result<(), solana_program_test::BanksClientError> {
    let ix_data = instruction::TransferPriorityFeeTips { lamports };

    let accounts = accounts::TransferPriorityFeeTips {
        config: *config,
        priority_fee_distribution_account: *priority_fee_distribution_account,
        from: from.pubkey(),
        system_program: system_program::id(),
    };

    let ix = solana_sdk::instruction::Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: ix_data.data(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, from],
        ctx.last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await
}

pub async fn call_migrate_tda_merkle_root_upload_authority(
    ctx: &ProgramTestContext,
    priority_fee_distribution_account: &Pubkey,
    merkle_root_upload_config: &Pubkey,
) -> Result<(), solana_program_test::BanksClientError> {
    let ix_data = instruction::MigrateTdaMerkleRootUploadAuthority {};

    let accounts = accounts::MigrateTdaMerkleRootUploadAuthority {
        priority_fee_distribution_account: *priority_fee_distribution_account,
        merkle_root_upload_config: *merkle_root_upload_config,
    };

    let ix = solana_sdk::instruction::Instruction {
        program_id: jito_priority_fee_distribution::id(),
        accounts: accounts.to_account_metas(None),
        data: ix_data.data(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await
}

pub async fn fund_account(
    ctx: &ProgramTestContext,
    from: &Pubkey,
    to: &Pubkey,
    lamports: u64,
) -> Result<(), solana_program_test::BanksClientError> {
    let ix = system_instruction::transfer(from, to, lamports);

    let tx =
        Transaction::new_signed_with_payer(&[ix], Some(from), &[&ctx.payer], ctx.last_blockhash);

    ctx.banks_client.process_transaction(tx).await
}
