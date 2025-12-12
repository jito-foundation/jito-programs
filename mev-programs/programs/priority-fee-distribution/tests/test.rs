pub mod helpers;

use anchor_lang::{error::ErrorCode as AnchorError, pubkey, InstructionData, ToAccountMetas};
use solana_program_test::{ProgramTest, ProgramTestContext};
use solana_sdk::{
    instruction::{Instruction, InstructionError},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::{Transaction, TransactionError},
};

use helpers::utils::*;

use jito_priority_fee_distribution::{
    accounts, instruction,
    state::{Config, MerkleRoot, MerkleRootUploadConfig, PriorityFeeDistributionAccount},
};

pub const CONFIG_ACCOUNT_SEED: &str = "CONFIG_ACCOUNT";
pub const PRIORITY_FEE_DISTRIBUTION_ACCOUNT_SEED: &str = "PF_DISTRIBUTION_ACCOUNT";
pub const PRIORITY_FEE_DISTRIBUTION_ACCOUNT_LEN: usize = 176;
pub const CLAIM_STATUS_SEED: &str = "CLAIM_STATUS";
pub const CLAIM_STATUS_LEN: usize = 48;
pub const ROOT_UPLOAD_CONFIG_SEED: &str = "ROOT_UPLOAD_CONFIG";
pub const JITO_MERKLE_UPLOAD_AUTHORITY: Pubkey =
    pubkey!("GZctHpWXmsZC1YHACTGGcHhYxjdRqQvTpYkb9LMvxDib");
pub const FUND_AMOUNT: u64 = 100 * solana_sdk::native_token::LAMPORTS_PER_SOL;

async fn get_test() -> ProgramTestContext {
    let mut test = ProgramTest::default();
    test.add_upgradeable_program_to_genesis(
        "jito_priority_fee_distribution",
        &jito_priority_fee_distribution::id(),
    );

    test.start_with_context().await
}

async fn initialize(ctx: &ProgramTestContext) -> Keypair {
    // given
    let initializer = generate_account(ctx, FUND_AMOUNT).await;
    let authority = generate_account(ctx, 0).await;
    let expired_funds_account = generate_account(ctx, FUND_AMOUNT).await;

    let (config_account_key, config_bump) = derive_config_account_address();

    let num_epochs_valid: u64 = 3;
    let max_validator_commission_bps: u16 = 1000;

    // then
    call_initialize(
        ctx,
        &authority,
        &expired_funds_account.pubkey(),
        num_epochs_valid,
        max_validator_commission_bps,
        config_bump,
        &config_account_key,
        &initializer,
    )
    .await
    .unwrap();

    authority
}

#[tokio::test]
async fn initialize_happy_path() {
    let ctx = get_test().await;

    // given
    let initializer = generate_account(&ctx, FUND_AMOUNT).await;
    let authority = generate_account(&ctx, 0).await;
    let expired_funds_account = generate_account(&ctx, FUND_AMOUNT).await;

    let (config_account_key, config_bump) = derive_config_account_address();

    let num_epochs_valid: u64 = 3;
    let max_validator_commission_bps: u16 = 1000;

    // then
    call_initialize(
        &ctx,
        &authority,
        &expired_funds_account.pubkey(),
        num_epochs_valid,
        max_validator_commission_bps,
        config_bump,
        &config_account_key,
        &initializer,
    )
    .await
    .unwrap();

    // expect
    let actual_config = get_deserialized_account::<Config>(&ctx, &config_account_key)
        .await
        .unwrap()
        .unwrap();

    let expected = Config {
        authority: authority.pubkey(),
        expired_funds_account: expired_funds_account.pubkey(),
        num_epochs_valid,
        max_validator_commission_bps,
        ..Default::default()
    };

    assert_config_state(&actual_config, &expected);
}

#[tokio::test]
async fn init_priority_fee_distribution_account_happy_path() {
    let ctx = get_test().await;

    initialize(&ctx).await;

    // given
    let setup =
        setup_init_tip_distribution_account(&ctx, &jito_priority_fee_distribution::id()).await;

    // then
    call_init_tip_distribution_account(
        &ctx,
        &jito_priority_fee_distribution::id(),
        setup.max_validator_commission_bps,
        &setup.validator_vote_account.pubkey(),
        &derive_config_account_address().0,
        &setup.validator_identity,
        &setup.validator_vote_account.pubkey(),
        &setup.priority_fee_distribution_account,
        setup.bump,
    )
    .await
    .unwrap();

    // expect
    let actual_fda = get_deserialized_account::<PriorityFeeDistributionAccount>(
        &ctx,
        &setup.priority_fee_distribution_account,
    )
    .await
    .unwrap()
    .unwrap();

    // only relevant fields are checked
    let expected_fda = PriorityFeeDistributionAccount {
        validator_vote_account: setup.validator_vote_account.pubkey(),
        merkle_root_upload_authority: setup.validator_vote_account.pubkey(),
        merkle_root: None,
        epoch_created_at: setup.epoch,
        validator_commission_bps: setup.max_validator_commission_bps,
        ..Default::default()
    };

    assert_distribution_account(&actual_fda, &expected_fda);
}

#[tokio::test]
async fn init_priority_fee_distribution_account_fails_with_invalid_commission() {
    let ctx = get_test().await;

    initialize(&ctx).await;

    // given
    let setup =
        setup_init_tip_distribution_account(&ctx, &jito_priority_fee_distribution::id()).await;

    // then
    let res = call_init_tip_distribution_account(
        &ctx,
        &jito_priority_fee_distribution::id(),
        setup.max_validator_commission_bps + 1,
        &setup.validator_vote_account.pubkey(),
        &derive_config_account_address().0,
        &setup.validator_identity,
        &setup.validator_vote_account.pubkey(),
        &setup.priority_fee_distribution_account,
        setup.bump,
    )
    .await;

    // expect
    let err = res.unwrap_err();
    assert_eq!(
        err.unwrap(),
        TransactionError::InstructionError(0, InstructionError::Custom(6009))
    );
}

#[tokio::test]
async fn close_priority_fee_distribution_account_happy_path() {
    let mut ctx = get_test().await;

    initialize(&ctx).await;

    // given
    let setup =
        setup_init_tip_distribution_account(&ctx, &jito_priority_fee_distribution::id()).await;

    call_init_tip_distribution_account(
        &ctx,
        &jito_priority_fee_distribution::id(),
        setup.max_validator_commission_bps,
        &setup.validator_vote_account.pubkey(),
        &derive_config_account_address().0,
        &setup.validator_identity,
        &setup.validator_vote_account.pubkey(),
        &setup.priority_fee_distribution_account,
        setup.bump,
    )
    .await
    .unwrap();

    let (config_pda, _) = derive_config_account_address();
    let cfg = get_deserialized_account::<Config>(&ctx, &config_pda)
        .await
        .unwrap()
        .unwrap();

    let fda_acc = get_deserialized_account::<PriorityFeeDistributionAccount>(
        &ctx,
        &setup.priority_fee_distribution_account,
    )
    .await
    .unwrap()
    .unwrap();

    let bal_start = get_balance(&ctx, &setup.validator_vote_account.pubkey())
        .await
        .unwrap();

    sleep_for_epochs(&mut ctx, 4).await;

    // close the account
    call_close_priority_fee_distribution_account(
        &ctx,
        &config_pda,
        &cfg.expired_funds_account,
        &setup.priority_fee_distribution_account,
        &setup.validator_vote_account.pubkey(),
        fda_acc.epoch_created_at,
    )
    .await
    .unwrap();

    let bal_end = get_balance(&ctx, &setup.validator_vote_account.pubkey())
        .await
        .unwrap();

    let rent = ctx.banks_client.get_rent().await.unwrap();
    let min_rent_exempt = rent.minimum_balance(PRIORITY_FEE_DISTRIBUTION_ACCOUNT_LEN);

    assert_eq!(bal_end.saturating_sub(bal_start), min_rent_exempt);

    // account should be closed
    let closed = get_deserialized_account::<PriorityFeeDistributionAccount>(
        &ctx,
        &setup.priority_fee_distribution_account,
    )
    .await
    .unwrap();

    assert!(closed.is_none());
}

#[tokio::test]
async fn upload_merkle_root_happy_path() {
    let mut ctx = get_test().await;

    initialize(&ctx).await;

    let MerkleSetup {
        root,
        priority_fee_distribution_account,
        validator_vote_account,
        epoch,
        max_validator_commission_bps,
        max_num_nodes,
        max_total_claim,
        ..
    } = setup_with_uploaded_merkle_root(&mut ctx, &jito_priority_fee_distribution::id()).await;

    let actual_fda = get_deserialized_account::<PriorityFeeDistributionAccount>(
        &ctx,
        &priority_fee_distribution_account,
    )
    .await
    .unwrap()
    .unwrap();

    let expected_fda = PriorityFeeDistributionAccount {
        validator_vote_account: validator_vote_account.pubkey(),
        merkle_root_upload_authority: validator_vote_account.pubkey(),
        merkle_root: Some(MerkleRoot {
            root,
            max_total_claim,
            max_num_nodes,
            total_funds_claimed: 0,
            num_nodes_claimed: 0,
        }),
        epoch_created_at: epoch,
        validator_commission_bps: max_validator_commission_bps,
        ..Default::default()
    };

    assert_distribution_account(&actual_fda, &expected_fda);
}

#[tokio::test]
async fn close_claim_status_fails_with_wrong_payer() {
    let mut ctx = get_test().await;

    initialize(&ctx).await;

    let MerkleSetup {
        amount0,
        priority_fee_distribution_account,
        tree,
        user0,
        user1,
        validator_vote_account,
        ..
    } = setup_with_uploaded_merkle_root(&mut ctx, &jito_priority_fee_distribution::id()).await;

    let index = 0;
    let proof = tree.get_proof(index);

    let claimant = &user0;

    let (claim_status, _) =
        derive_claim_status_account_address(&claimant.pubkey(), &priority_fee_distribution_account);

    call_claim(
        &ctx,
        &priority_fee_distribution_account,
        &validator_vote_account,
        &claim_status,
        &user0,
        &user1,
        amount0,
        proof,
    )
    .await
    .unwrap();

    sleep_for_epochs(&mut ctx, 4).await;

    let wrong_payer = Keypair::new();

    let close_ix = instruction::CloseClaimStatus {};

    let close_accounts = accounts::CloseClaimStatus {
        claim_status,
        claim_status_payer: wrong_payer.pubkey(),
    };

    let close_instruction = Instruction {
        program_id: jito_priority_fee_distribution::id(),
        accounts: close_accounts.to_account_metas(None),
        data: close_ix.data(),
    };

    let close_tx = Transaction::new_signed_with_payer(
        &[close_instruction],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );

    let res = ctx.banks_client.process_transaction(close_tx).await;

    let err = res.unwrap_err();
    assert_eq!(
        err.unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AnchorError::ConstraintRaw as u32),
        )
    );
}

#[tokio::test]
async fn close_claim_status_premature_fails() {
    let mut ctx = get_test().await;

    initialize(&ctx).await;

    let MerkleSetup {
        amount0,
        priority_fee_distribution_account,
        tree,
        user0,
        user1,
        validator_vote_account,
        ..
    } = setup_with_uploaded_merkle_root(&mut ctx, &jito_priority_fee_distribution::id()).await;

    let index = 0;
    let proof = tree.get_proof(index);
    let claimant = &user0;

    let (claim_status, _) =
        derive_claim_status_account_address(&claimant.pubkey(), &priority_fee_distribution_account);

    call_claim(
        &ctx,
        &priority_fee_distribution_account,
        &validator_vote_account,
        &claim_status,
        &claimant,
        &user1,
        amount0,
        proof,
    )
    .await
    .unwrap();

    // should usually wait a few epochs after claiming to close the ClaimAccount
    // since we didn't wait, we cannot close the ClaimStatus account
    let bal_start = get_balance(&ctx, &user1.pubkey()).await.unwrap();

    let res = call_close_claim_status(&ctx, &claim_status, &user1).await;

    let err = res.unwrap_err();
    assert_eq!(
        err.unwrap(),
        TransactionError::InstructionError(0, InstructionError::Custom(6011),)
    );

    let bal_end = ctx.banks_client.get_balance(user1.pubkey()).await.unwrap();

    assert_eq!(bal_start, bal_end);
}

#[tokio::test]
async fn close_claim_status_fails_when_user_tries_to_drain_tip_distribution_account() {
    let mut ctx = get_test().await;

    initialize(&ctx).await;

    let MerkleSetup {
        amount0,
        priority_fee_distribution_account,
        tree,
        user0,
        user1,
        validator_vote_account,
        ..
    } = setup_with_uploaded_merkle_root(&mut ctx, &jito_priority_fee_distribution::id()).await;

    let index = 0;
    let proof = tree.get_proof(index);
    let claimant = &user0;

    let (claim_status, _) =
        derive_claim_status_account_address(&claimant.pubkey(), &priority_fee_distribution_account);

    call_claim(
        &ctx,
        &priority_fee_distribution_account,
        &validator_vote_account,
        &claim_status,
        &claimant,
        &user1,
        amount0,
        proof.clone(),
    )
    .await
    .unwrap();

    // wait for TDA to expire
    sleep_for_epochs(&mut ctx, 3).await;

    // close claim status (user1 is payer)
    call_close_claim_status(&ctx, &claim_status, &user1)
        .await
        .unwrap();

    // try to claim second time, this should fail since the TDA has expired
    let res = call_claim(
        &ctx,
        &priority_fee_distribution_account,
        &validator_vote_account,
        &claim_status,
        &claimant,
        &user1,
        amount0,
        proof,
    )
    .await;

    let err = res.unwrap_err();
    assert_eq!(
        err.unwrap(),
        TransactionError::InstructionError(0, InstructionError::Custom(6004))
    );
}

#[tokio::test]
async fn close_claim_status_happy_path() {
    let mut ctx = get_test().await;

    initialize(&ctx).await;

    let MerkleSetup {
        amount0,
        priority_fee_distribution_account,
        tree,
        user0,
        user1,
        validator_vote_account,
        ..
    } = setup_with_uploaded_merkle_root(&mut ctx, &jito_priority_fee_distribution::id()).await;

    let index = 0;
    let proof = tree.get_proof(index);
    let claimant = &user0;

    let (claim_status, _) =
        derive_claim_status_account_address(&claimant.pubkey(), &priority_fee_distribution_account);

    call_claim(
        &ctx,
        &priority_fee_distribution_account,
        &validator_vote_account,
        &claim_status,
        claimant,
        &user1,
        amount0,
        proof,
    )
    .await
    .unwrap();

    sleep_for_epochs(&mut ctx, 4).await;

    let bal_start = get_balance(&ctx, &user1.pubkey()).await.unwrap();

    let claim_acc = get_account(&ctx, &claim_status).await.unwrap().unwrap();

    let rent = ctx.banks_client.get_rent().await.unwrap();
    let min_rent_exempt = rent.minimum_balance(claim_acc.data.len());

    call_close_claim_status(&ctx, &claim_status, &user1)
        .await
        .unwrap();

    let bal_end = get_balance(&ctx, &user1.pubkey()).await.unwrap();

    assert_eq!(bal_end.saturating_sub(bal_start), min_rent_exempt);
}

#[tokio::test]
async fn close_claim_status_works_even_if_tip_distribution_account_already_closed() {
    let mut ctx = get_test().await;

    initialize(&ctx).await;

    let MerkleSetup {
        amount0,
        priority_fee_distribution_account,
        tree,
        user0,
        user1,
        validator_vote_account,
        ..
    } = setup_with_uploaded_merkle_root(&mut ctx, &jito_priority_fee_distribution::id()).await;

    let index = 0;
    let proof = tree.get_proof(index);
    let claimant = &user0;

    let (claim_status, _) =
        derive_claim_status_account_address(&claimant.pubkey(), &priority_fee_distribution_account);

    call_claim(
        &ctx,
        &priority_fee_distribution_account,
        &validator_vote_account,
        &claim_status,
        claimant,
        &user1,
        amount0,
        proof,
    )
    .await
    .unwrap();

    sleep_for_epochs(&mut ctx, 3).await;

    let config_pda = derive_config_account_address().0;
    let cfg = get_deserialized_account::<Config>(&ctx, &config_pda)
        .await
        .unwrap()
        .unwrap();

    let tda = get_deserialized_account::<PriorityFeeDistributionAccount>(
        &ctx,
        &priority_fee_distribution_account,
    )
    .await
    .unwrap()
    .unwrap();

    call_close_priority_fee_distribution_account(
        &ctx,
        &config_pda,
        &cfg.expired_funds_account,
        &priority_fee_distribution_account,
        &validator_vote_account.pubkey(),
        tda.epoch_created_at,
    )
    .await
    .unwrap();

    let bal_start = get_balance(&ctx, &user1.pubkey()).await.unwrap();

    let claim_acc = get_account(&ctx, &claim_status).await.unwrap().unwrap();

    let rent = ctx.banks_client.get_rent().await.unwrap();
    let min_rent_exempt = rent.minimum_balance(claim_acc.data.len());

    call_close_claim_status(&ctx, &claim_status, &user1)
        .await
        .unwrap();

    let bal_end = get_balance(&ctx, &user1.pubkey()).await.unwrap();

    assert_eq!(bal_end.saturating_sub(bal_start), min_rent_exempt);
}

#[tokio::test]
async fn claim_happy_path() {
    let mut ctx = get_test().await;

    initialize(&ctx).await;

    let MerkleSetup {
        amount0,
        pre_balance0,
        priority_fee_distribution_account,
        tree,
        user0,
        user1,
        validator_vote_account,
        ..
    } = setup_with_uploaded_merkle_root(&mut ctx, &jito_priority_fee_distribution::id()).await;

    let index = 0;
    let proof = tree.get_proof(index);

    let (claim_status, _) =
        derive_claim_status_account_address(&user0.pubkey(), &priority_fee_distribution_account);

    call_claim(
        &ctx,
        &priority_fee_distribution_account,
        &validator_vote_account,
        &claim_status,
        &user0,
        &user1,
        amount0,
        proof,
    )
    .await
    .unwrap();

    let bal = get_balance(&ctx, &user0.pubkey()).await.unwrap();
    assert_eq!(bal, pre_balance0 + amount0);
}

#[tokio::test]
async fn claim_fails_if_tda_merkle_root_upload_authority_not_signer() {
    let mut ctx = get_test().await;

    initialize(&ctx).await;

    let MerkleSetup {
        amount0,
        priority_fee_distribution_account,
        tree,
        user0,
        user1,
        ..
    } = setup_with_uploaded_merkle_root(&mut ctx, &jito_priority_fee_distribution::id()).await;

    let index = 0;
    let proof = tree.get_proof(index);

    let (claim_status, _) =
        derive_claim_status_account_address(&user0.pubkey(), &priority_fee_distribution_account);

    let bad_authority = Keypair::new();

    let res = call_claim(
        &ctx,
        &priority_fee_distribution_account,
        &bad_authority,
        &claim_status,
        &user0,
        &user1,
        amount0,
        proof,
    )
    .await;

    let err = res.unwrap_err();
    assert_eq!(
        err.unwrap(),
        TransactionError::InstructionError(0, InstructionError::Custom(6014))
    );
}

#[tokio::test]
async fn initialize_merkle_root_upload_config_happy_path() {
    let ctx = get_test().await;

    let authority = initialize(&ctx).await;

    let (config_account_key, _) = derive_config_account_address();

    let (merkle_root_upload_config_key, merkle_root_upload_config_bump) =
        derive_merkle_root_upload_config_address();

    let original_authority = Keypair::new();
    let override_authority = Keypair::new();

    call_initialize_merkle_root_upload_config(
        &ctx,
        &config_account_key,
        &merkle_root_upload_config_key,
        &authority,
        &override_authority,
        &original_authority.pubkey(),
    )
    .await
    .unwrap();

    let merkle_root_upload_config =
        get_deserialized_account::<MerkleRootUploadConfig>(&ctx, &merkle_root_upload_config_key)
            .await
            .unwrap()
            .unwrap();

    assert_eq!(
        merkle_root_upload_config.bump,
        merkle_root_upload_config_bump
    );
    assert_eq!(
        merkle_root_upload_config.override_authority,
        override_authority.pubkey()
    );
    assert_eq!(
        merkle_root_upload_config.original_upload_authority,
        original_authority.pubkey()
    );
}

#[tokio::test]
async fn update_merkle_root_upload_config_happy_path() {
    let ctx = get_test().await;

    let authority = initialize(&ctx).await;

    let (config_account_key, _) = derive_config_account_address();

    let (merkle_root_upload_config_key, _) = derive_merkle_root_upload_config_address();
    let original_authority = Keypair::new();
    let override_authority = Keypair::new();
    let new_override_authority = Keypair::new();

    call_initialize_merkle_root_upload_config(
        &ctx,
        &config_account_key,
        &merkle_root_upload_config_key,
        &authority,
        &override_authority,
        &original_authority.pubkey(),
    )
    .await
    .unwrap();

    call_update_merkle_root_upload_config(
        &ctx,
        &config_account_key,
        &merkle_root_upload_config_key,
        &authority,
        &new_override_authority.pubkey(),
        &JITO_MERKLE_UPLOAD_AUTHORITY,
    )
    .await
    .unwrap();

    let merkle_root_upload_config =
        get_deserialized_account::<MerkleRootUploadConfig>(&ctx, &merkle_root_upload_config_key)
            .await
            .unwrap()
            .unwrap();

    assert_eq!(
        merkle_root_upload_config.override_authority,
        new_override_authority.pubkey()
    );
    assert_eq!(
        merkle_root_upload_config.original_upload_authority,
        JITO_MERKLE_UPLOAD_AUTHORITY
    );
}

#[tokio::test]
async fn migrate_tda_merkle_root_upload_authority_happy_path() {
    let ctx = get_test().await;

    let authority = initialize(&ctx).await;

    let setup =
        setup_init_tip_distribution_account(&ctx, &jito_priority_fee_distribution::id()).await;

    let override_authority = Keypair::new();
    let (config_account_key, _) = derive_config_account_address();
    let (merkle_root_upload_config_key, _) = derive_merkle_root_upload_config_address();

    call_init_tip_distribution_account(
        &ctx,
        &jito_priority_fee_distribution::id(),
        setup.max_validator_commission_bps,
        &JITO_MERKLE_UPLOAD_AUTHORITY,
        &derive_config_account_address().0,
        &setup.validator_identity,
        &setup.validator_vote_account.pubkey(),
        &setup.priority_fee_distribution_account,
        setup.bump,
    )
    .await
    .unwrap();

    call_initialize_merkle_root_upload_config(
        &ctx,
        &config_account_key,
        &merkle_root_upload_config_key,
        &authority,
        &override_authority,
        &JITO_MERKLE_UPLOAD_AUTHORITY,
    )
    .await
    .unwrap();

    call_migrate_tda_merkle_root_upload_authority(
        &ctx,
        &setup.priority_fee_distribution_account,
        &merkle_root_upload_config_key,
    )
    .await
    .unwrap();

    let tda = get_deserialized_account::<PriorityFeeDistributionAccount>(
        &ctx,
        &setup.priority_fee_distribution_account,
    )
    .await
    .unwrap()
    .unwrap();

    let mru =
        get_deserialized_account::<MerkleRootUploadConfig>(&ctx, &merkle_root_upload_config_key)
            .await
            .unwrap()
            .unwrap();

    assert_eq!(tda.merkle_root_upload_authority, mru.override_authority);
}

#[tokio::test]
async fn migrate_tda_merkle_root_upload_authority_should_error_if_tda_not_jito_authority() {
    let ctx = get_test().await;

    let authority = initialize(&ctx).await;

    let setup =
        setup_init_tip_distribution_account(&ctx, &jito_priority_fee_distribution::id()).await;

    call_init_tip_distribution_account(
        &ctx,
        &jito_priority_fee_distribution::id(),
        setup.max_validator_commission_bps,
        &setup.validator_vote_account.pubkey(),
        &derive_config_account_address().0,
        &setup.validator_identity,
        &setup.validator_vote_account.pubkey(),
        &setup.priority_fee_distribution_account,
        setup.bump,
    )
    .await
    .unwrap();

    let override_authority = Keypair::new();
    let (config_account_key, _) = derive_config_account_address();
    let (merkle_root_upload_config_key, _) = derive_merkle_root_upload_config_address();

    call_initialize_merkle_root_upload_config(
        &ctx,
        &config_account_key,
        &merkle_root_upload_config_key,
        &authority,
        &override_authority,
        &JITO_MERKLE_UPLOAD_AUTHORITY,
    )
    .await
    .unwrap();

    let res = call_migrate_tda_merkle_root_upload_authority(
        &ctx,
        &setup.priority_fee_distribution_account,
        &merkle_root_upload_config_key,
    )
    .await;

    let err = res.unwrap_err();
    assert_eq!(
        err.unwrap(),
        TransactionError::InstructionError(0, InstructionError::Custom(6015))
    );
}

#[tokio::test]
async fn migrate_tda_merkle_root_upload_authority_should_error_if_merkle_root_already_uploaded() {
    let mut ctx = get_test().await;

    let authority = initialize(&ctx).await;

    let setup =
        setup_init_tip_distribution_account(&ctx, &jito_priority_fee_distribution::id()).await;

    call_init_tip_distribution_account(
        &ctx,
        &jito_priority_fee_distribution::id(),
        setup.max_validator_commission_bps,
        &JITO_MERKLE_UPLOAD_AUTHORITY,
        &derive_config_account_address().0,
        &setup.validator_identity,
        &setup.validator_vote_account.pubkey(),
        &setup.priority_fee_distribution_account,
        setup.bump,
    )
    .await
    .unwrap();

    let override_authority = Keypair::new();
    let (config_account_key, _) = derive_config_account_address();
    let (merkle_root_upload_config_key, _) = derive_merkle_root_upload_config_address();

    call_initialize_merkle_root_upload_config(
        &ctx,
        &config_account_key,
        &merkle_root_upload_config_key,
        &authority,
        &override_authority,
        &JITO_MERKLE_UPLOAD_AUTHORITY,
    )
    .await
    .unwrap();

    let MerkleSetup {
        priority_fee_distribution_account,
        ..
    } = setup_with_uploaded_merkle_root(&mut ctx, &jito_priority_fee_distribution::id()).await;

    let res = call_migrate_tda_merkle_root_upload_authority(
        &ctx,
        &priority_fee_distribution_account,
        &merkle_root_upload_config_key,
    )
    .await;

    let err = res.unwrap_err();
    assert_eq!(
        err.unwrap(),
        TransactionError::InstructionError(0, InstructionError::Custom(6015))
    );
}

#[tokio::test]
async fn transfer_priority_fee_tips_should_not_make_any_transfer() {
    let mut ctx = get_test().await;

    initialize(&ctx).await;

    let random_payer =
        generate_account(&ctx, 10 * solana_sdk::native_token::LAMPORTS_PER_SOL).await;

    sleep_for_epochs(&mut ctx, 1).await;

    let setup =
        setup_init_tip_distribution_account(&ctx, &jito_priority_fee_distribution::id()).await;

    call_init_tip_distribution_account(
        &ctx,
        &jito_priority_fee_distribution::id(),
        setup.max_validator_commission_bps,
        &setup.validator_vote_account.pubkey(),
        &derive_config_account_address().0,
        &setup.validator_identity,
        &setup.validator_vote_account.pubkey(),
        &setup.priority_fee_distribution_account,
        setup.bump,
    )
    .await
    .unwrap();

    let lamports_to_transfer: u64 =
        (27u128 * solana_sdk::native_token::LAMPORTS_PER_SOL as u128 / 10u128) as u64;

    let dist_acc_before = get_account(&ctx, &setup.priority_fee_distribution_account)
        .await
        .unwrap()
        .unwrap();

    let lamports_before = dist_acc_before.lamports;

    let before_struct = get_deserialized_account::<PriorityFeeDistributionAccount>(
        &ctx,
        &setup.priority_fee_distribution_account,
    )
    .await
    .unwrap()
    .unwrap();

    call_transfer_priority_fee_tips(
        &ctx,
        &jito_priority_fee_distribution::id(),
        &derive_config_account_address().0,
        &setup.priority_fee_distribution_account,
        &random_payer,
        lamports_to_transfer,
    )
    .await
    .unwrap();

    let dist_acc_after = ctx
        .banks_client
        .get_account(setup.priority_fee_distribution_account)
        .await
        .unwrap()
        .unwrap();
    let lamports_after = dist_acc_after.lamports;

    let after_struct = get_deserialized_account::<PriorityFeeDistributionAccount>(
        &ctx,
        &setup.priority_fee_distribution_account,
    )
    .await
    .unwrap()
    .unwrap();

    assert_eq!(lamports_after.saturating_sub(lamports_before), 0);

    assert_eq!(
        after_struct
            .total_lamports_transferred
            .saturating_sub(before_struct.total_lamports_transferred),
        lamports_to_transfer
    );
}

#[tokio::test]
async fn transfer_priority_fee_tips_after_go_live_happy_path() {
    let mut ctx = get_test().await;

    let initializer = generate_account(&ctx, FUND_AMOUNT).await;
    let authority = generate_account(&ctx, FUND_AMOUNT).await;
    let expired_funds = generate_account(&ctx, FUND_AMOUNT).await;

    let (config_pda, config_bump) = derive_config_account_address();

    call_initialize(
        &ctx,
        &authority,
        &expired_funds.pubkey(),
        3u64,
        1000u16,
        config_bump,
        &config_pda,
        &initializer,
    )
    .await
    .unwrap();

    let cfg = get_deserialized_account::<Config>(&ctx, &config_pda)
        .await
        .unwrap()
        .unwrap();

    let clock = ctx
        .banks_client
        .get_sysvar::<solana_sdk::sysvar::clock::Clock>()
        .await
        .unwrap();
    let current_epoch = clock.epoch;

    let mut new_cfg = cfg;
    new_cfg.go_live_epoch = current_epoch;

    call_update_config(&ctx, new_cfg, &authority, &config_pda)
        .await
        .unwrap();

    let random_payer =
        generate_account(&ctx, 10 * solana_sdk::native_token::LAMPORTS_PER_SOL).await;

    sleep_for_epochs(&mut ctx, 1).await;

    let setup =
        setup_init_tip_distribution_account(&ctx, &jito_priority_fee_distribution::id()).await;

    call_init_tip_distribution_account(
        &ctx,
        &jito_priority_fee_distribution::id(),
        setup.max_validator_commission_bps,
        &setup.validator_vote_account.pubkey(),
        &config_pda,
        &setup.validator_identity,
        &setup.validator_vote_account.pubkey(),
        &setup.priority_fee_distribution_account,
        setup.bump,
    )
    .await
    .unwrap();

    let lamports_to_transfer: u64 =
        (27u128 * solana_sdk::native_token::LAMPORTS_PER_SOL as u128 / 10u128) as u64;

    let dist_acc_before = get_account(&ctx, &setup.priority_fee_distribution_account)
        .await
        .unwrap()
        .unwrap();

    let lamports_before = dist_acc_before.lamports;

    call_transfer_priority_fee_tips(
        &ctx,
        &jito_priority_fee_distribution::id(),
        &config_pda,
        &setup.priority_fee_distribution_account,
        &random_payer,
        lamports_to_transfer,
    )
    .await
    .unwrap();

    let dist_acc_after = get_account(&ctx, &setup.priority_fee_distribution_account)
        .await
        .unwrap()
        .unwrap();

    let lamports_after = dist_acc_after.lamports;

    assert_eq!(
        lamports_after.saturating_sub(lamports_before),
        lamports_to_transfer
    );
}

#[tokio::test]
async fn transfer_priority_fee_tips_after_go_live_distribution_account_from_old_epoch() {
    let mut ctx = get_test().await;

    let initializer = generate_account(&ctx, FUND_AMOUNT).await;
    let authority = generate_account(&ctx, FUND_AMOUNT).await;
    let expired_funds = generate_account(&ctx, FUND_AMOUNT).await;

    let (config_pda, config_bump) = derive_config_account_address();

    call_initialize(
        &ctx,
        &authority,
        &expired_funds.pubkey(),
        3u64,
        1000u16,
        config_bump,
        &config_pda,
        &initializer,
    )
    .await
    .unwrap();

    let cfg = get_deserialized_account::<Config>(&ctx, &config_pda)
        .await
        .unwrap()
        .unwrap();

    let clock = ctx
        .banks_client
        .get_sysvar::<solana_sdk::sysvar::clock::Clock>()
        .await
        .unwrap();
    let current_epoch = clock.epoch;

    let mut new_cfg = cfg;
    new_cfg.go_live_epoch = current_epoch;

    call_update_config(&ctx, new_cfg, &authority, &config_pda)
        .await
        .unwrap();

    let setup =
        setup_init_tip_distribution_account(&ctx, &jito_priority_fee_distribution::id()).await;

    call_init_tip_distribution_account(
        &ctx,
        &jito_priority_fee_distribution::id(),
        setup.max_validator_commission_bps,
        &setup.validator_vote_account.pubkey(),
        &config_pda,
        &setup.validator_identity,
        &setup.validator_vote_account.pubkey(),
        &setup.priority_fee_distribution_account,
        setup.bump,
    )
    .await
    .unwrap();

    let random_payer =
        generate_account(&ctx, 10 * solana_sdk::native_token::LAMPORTS_PER_SOL).await;

    sleep_for_epochs(&mut ctx, 1).await;

    let lamports_to_transfer: u64 =
        (13u128 * solana_sdk::native_token::LAMPORTS_PER_SOL as u128 / 10u128) as u64;

    let res = call_transfer_priority_fee_tips(
        &ctx,
        &jito_priority_fee_distribution::id(),
        &config_pda,
        &setup.priority_fee_distribution_account,
        &random_payer,
        lamports_to_transfer,
    )
    .await;

    let err = res.unwrap_err();
    assert_eq!(
        err.unwrap(),
        TransactionError::InstructionError(0, InstructionError::Custom(6000))
    );
}
