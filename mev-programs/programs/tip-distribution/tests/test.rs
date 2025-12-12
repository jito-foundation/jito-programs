pub mod helpers;

use anchor_lang::{error::ErrorCode as AnchorError, pubkey};
use jito_tip_distribution::state::{
    Config, MerkleRoot, MerkleRootUploadConfig, TipDistributionAccount,
};
use solana_program_test::{ProgramTest, ProgramTestContext};
use solana_sdk::{
    instruction::InstructionError,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::TransactionError,
};

use crate::helpers::utils::*;

pub const CONFIG_ACCOUNT_SEED: &str = "CONFIG_ACCOUNT";
pub const TIP_DISTRIBUTION_ACCOUNT_SEED: &str = "TIP_DISTRIBUTION_ACCOUNT";
pub const TIP_DISTRIBUTION_ACCOUNT_LEN: usize = 168;
pub const CLAIM_STATUS_SEED: &str = "CLAIM_STATUS";
pub const CLAIM_STATUS_LEN: usize = 104;
pub const ROOT_UPLOAD_CONFIG_SEED: &str = "ROOT_UPLOAD_CONFIG";
pub const JITO_MERKLE_UPLOAD_AUTHORITY: Pubkey =
    pubkey!("GZctHpWXmsZC1YHACTGGcHhYxjdRqQvTpYkb9LMvxDib");

async fn get_test() -> ProgramTestContext {
    let mut test = ProgramTest::default();
    test.add_upgradeable_program_to_genesis("jito_tip_distribution", &jito_tip_distribution::id());

    test.start_with_context().await
}

async fn initialize(ctx: &ProgramTestContext) -> Keypair {
    let initializer = generate_account(ctx, 100_000_000_000_000).await;
    let authority = generate_account(ctx, 100_000_000_000_000).await;
    let expired_funds_account = generate_account(ctx, 100_000_000_000_000).await;

    let (config_account_key, config_bump) = derive_config_account_address();

    let num_epochs_valid: u64 = 3;
    let max_validator_commission_bps: u16 = 1000;

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
    let initializer = generate_account(&ctx, 100_000_000_000_000).await;
    let authority = generate_account(&ctx, 0).await;
    let expired_funds_account = generate_account(&ctx, 0).await;

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
async fn init_tip_distribution_account_happy_path() {
    let ctx = get_test().await;

    initialize(&ctx).await;

    // given
    let setup = setup_init_tip_distribution_account(&ctx, &jito_tip_distribution::id()).await;

    // then
    call_init_tip_distribution_account(
        &ctx,
        &jito_tip_distribution::id(),
        setup.max_validator_commission_bps,
        &setup.validator_vote_account.pubkey(),
        &derive_config_account_address().0,
        &setup.validator_identity,
        &setup.validator_vote_account.pubkey(),
        &setup.tip_distribution_account,
        setup.bump,
    )
    .await
    .unwrap();

    let actual =
        get_deserialized_account::<TipDistributionAccount>(&ctx, &setup.tip_distribution_account)
            .await
            .unwrap()
            .unwrap();

    let expected = TipDistributionAccount {
        validator_vote_account: setup.validator_vote_account.pubkey(),
        epoch_created_at: setup.epoch,
        merkle_root: None,
        merkle_root_upload_authority: setup.validator_vote_account.pubkey(),
        validator_commission_bps: setup.max_validator_commission_bps,
        ..Default::default()
    };
    assert_distribution_account(&actual, &expected);
}

#[tokio::test]
async fn init_tip_distribution_account_fails_with_max_validator_commission_fee_bps_exceeded() {
    let ctx = get_test().await;

    initialize(&ctx).await;

    // given
    let setup = setup_init_tip_distribution_account(&ctx, &jito_tip_distribution::id()).await;

    // then
    let res = call_init_tip_distribution_account(
        &ctx,
        &jito_tip_distribution::id(),
        setup.max_validator_commission_bps + 1,
        &setup.validator_vote_account.pubkey(),
        &derive_config_account_address().0,
        &setup.validator_identity,
        &setup.validator_vote_account.pubkey(),
        &setup.tip_distribution_account,
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
async fn close_tip_distribution_account_happy_path() {
    let mut ctx = get_test().await;

    initialize(&ctx).await;

    // given
    let setup = setup_init_tip_distribution_account(&ctx, &jito_tip_distribution::id()).await;

    call_init_tip_distribution_account(
        &ctx,
        &jito_tip_distribution::id(),
        setup.max_validator_commission_bps,
        &setup.validator_vote_account.pubkey(),
        &derive_config_account_address().0,
        &setup.validator_identity,
        &setup.validator_vote_account.pubkey(),
        &setup.tip_distribution_account,
        setup.bump,
    )
    .await
    .unwrap();

    let (config_pda, _) = derive_config_account_address();
    let cfg = get_deserialized_account::<Config>(&ctx, &config_pda)
        .await
        .unwrap()
        .unwrap();

    let tda =
        get_deserialized_account::<TipDistributionAccount>(&ctx, &setup.tip_distribution_account)
            .await
            .unwrap()
            .unwrap();

    let bal_start = get_balance(&ctx, &setup.validator_vote_account.pubkey())
        .await
        .unwrap();

    sleep_for_epochs(&mut ctx, 4).await;

    // close the account
    call_close_tip_distribution_account(
        &ctx,
        &config_pda,
        &cfg.expired_funds_account,
        &setup.tip_distribution_account,
        &setup.validator_vote_account.pubkey(), //funds transferred to this account
        tda.epoch_created_at,
    )
    .await
    .unwrap();

    let bal_end = get_balance(&ctx, &setup.validator_vote_account.pubkey())
        .await
        .unwrap();

    let rent = ctx.banks_client.get_rent().await.unwrap();
    let min_rent_exempt = rent.minimum_balance(TIP_DISTRIBUTION_ACCOUNT_LEN);

    assert_eq!(bal_end - bal_start, min_rent_exempt);

    // the account is closed
    let acc_opt =
        get_deserialized_account::<TipDistributionAccount>(&ctx, &setup.tip_distribution_account)
            .await
            .unwrap();
    assert!(acc_opt.is_none());
}

#[tokio::test]
async fn upload_merkle_root_happy_path() {
    let mut ctx = get_test().await;

    initialize(&ctx).await;

    let MerkleSetup {
        root,
        tip_distribution_account,
        validator_vote_account,
        epoch,
        max_validator_commission_bps,
        max_num_nodes,
        max_total_claim,
        ..
    } = setup_with_uploaded_merkle_root(&mut ctx, &jito_tip_distribution::id()).await;

    let actual =
        get_deserialized_account::<TipDistributionAccount>(&ctx, &tip_distribution_account)
            .await
            .unwrap()
            .unwrap();

    let expected = TipDistributionAccount {
        validator_vote_account: validator_vote_account.pubkey(),
        epoch_created_at: epoch,
        merkle_root: Some(MerkleRoot {
            root,
            max_total_claim,
            max_num_nodes,
            total_funds_claimed: 0,
            num_nodes_claimed: 0,
        }),
        merkle_root_upload_authority: validator_vote_account.pubkey(),
        validator_commission_bps: max_validator_commission_bps,
        ..Default::default()
    };
    assert_distribution_account(&actual, &expected);
}

#[tokio::test]
async fn close_claim_status_fails_incorrect_claimant() {
    let mut ctx = get_test().await;

    initialize(&ctx).await;

    let MerkleSetup {
        amount0,
        tip_distribution_account,
        tree,
        user0,
        user1,
        validator_vote_account,
        ..
    } = setup_with_uploaded_merkle_root(&mut ctx, &jito_tip_distribution::id()).await;

    let index = 0;
    let proof = tree.get_proof(index);
    let claimant = &user0;

    let (claim_status, _) =
        derive_claim_status_account_address(&claimant.pubkey(), &tip_distribution_account);

    call_claim(
        &ctx,
        &tip_distribution_account,
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

    let wrong_payer = Keypair::new();

    let res = call_close_claim_status(&ctx, &claim_status, &wrong_payer).await;

    let err = res.unwrap_err();
    assert_eq!(
        err.unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AnchorError::ConstraintRaw as u32)
        )
    );
}

#[tokio::test]
async fn close_claim_status_fails_before_tip_distribution_account_has_expired_with_premature_close_claim_status(
) {
    let mut ctx = get_test().await;

    initialize(&ctx).await;

    let MerkleSetup {
        amount0,
        tip_distribution_account,
        tree,
        user0,
        user1,
        validator_vote_account,
        ..
    } = setup_with_uploaded_merkle_root(&mut ctx, &jito_tip_distribution::id()).await;

    let index = 0;
    let proof = tree.get_proof(index);
    let claimant = &user0;

    let (claim_status, _) =
        derive_claim_status_account_address(&claimant.pubkey(), &tip_distribution_account);

    call_claim(
        &ctx,
        &tip_distribution_account,
        &validator_vote_account,
        &claim_status,
        claimant,
        &user1,
        amount0,
        proof,
    )
    .await
    .unwrap();

    let bal_start = get_balance(&ctx, &user1.pubkey()).await.unwrap();

    let res = call_close_claim_status(&ctx, &claim_status, &user1).await;

    let err = res.unwrap_err();
    assert_eq!(
        err.unwrap(),
        TransactionError::InstructionError(0, InstructionError::Custom(6011))
    );

    let bal_end = get_balance(&ctx, &user1.pubkey()).await.unwrap();
    assert_eq!(bal_end, bal_start);
}

#[tokio::test]
async fn close_claim_status_fails_when_user_tries_to_drain_tip_distribution_account() {
    let mut ctx = get_test().await;

    initialize(&ctx).await;

    let MerkleSetup {
        amount0,
        tip_distribution_account,
        tree,
        user0,
        user1,
        validator_vote_account,
        ..
    } = setup_with_uploaded_merkle_root(&mut ctx, &jito_tip_distribution::id()).await;

    let index = 0;
    let proof = tree.get_proof(index);
    let claimant = &user0;

    let (claim_status, _) =
        derive_claim_status_account_address(&claimant.pubkey(), &tip_distribution_account);

    call_claim(
        &ctx,
        &tip_distribution_account,
        &validator_vote_account,
        &claim_status,
        claimant,
        &user1,
        amount0,
        proof.clone(),
    )
    .await
    .unwrap();

    sleep_for_epochs(&mut ctx, 3).await;

    call_close_claim_status(&ctx, &claim_status, &user1)
        .await
        .unwrap();

    let res = call_claim(
        &ctx,
        &tip_distribution_account,
        &validator_vote_account,
        &claim_status,
        claimant,
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
        tip_distribution_account,
        tree,
        user0,
        user1,
        validator_vote_account,
        ..
    } = setup_with_uploaded_merkle_root(&mut ctx, &jito_tip_distribution::id()).await;

    let index = 0;
    let proof = tree.get_proof(index);
    let claimant = &user0;

    let (claim_status, _) =
        derive_claim_status_account_address(&claimant.pubkey(), &tip_distribution_account);

    call_claim(
        &ctx,
        &tip_distribution_account,
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

    call_close_claim_status(&ctx, &claim_status, &user1)
        .await
        .unwrap();

    let bal_end = get_balance(&ctx, &user1.pubkey()).await.unwrap();

    let rent = ctx.banks_client.get_rent().await.unwrap();
    let min_rent_exempt = rent.minimum_balance(CLAIM_STATUS_LEN);

    assert_eq!(bal_end - bal_start, min_rent_exempt);
}

#[tokio::test]
async fn close_claim_status_works_even_if_tip_distribution_account_already_closed() {
    let mut ctx = get_test().await;

    initialize(&ctx).await;

    let MerkleSetup {
        amount0,
        tip_distribution_account,
        tree,
        user0,
        user1,
        validator_vote_account,
        ..
    } = setup_with_uploaded_merkle_root(&mut ctx, &jito_tip_distribution::id()).await;

    let index = 0;
    let proof = tree.get_proof(index);
    let claimant = &user0;

    let (claim_status, _) =
        derive_claim_status_account_address(&claimant.pubkey(), &tip_distribution_account);

    call_claim(
        &ctx,
        &tip_distribution_account,
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

    let (config_pda, _) = derive_config_account_address();
    let cfg = get_deserialized_account::<Config>(&ctx, &config_pda)
        .await
        .unwrap()
        .unwrap();

    let tda = get_deserialized_account::<TipDistributionAccount>(&ctx, &tip_distribution_account)
        .await
        .unwrap()
        .unwrap();

    call_close_tip_distribution_account(
        &ctx,
        &config_pda,
        &cfg.expired_funds_account,
        &tip_distribution_account,
        &validator_vote_account.pubkey(),
        tda.epoch_created_at,
    )
    .await
    .unwrap();

    let bal_start = get_balance(&ctx, &user1.pubkey()).await.unwrap();

    call_close_claim_status(&ctx, &claim_status, &user1)
        .await
        .unwrap();

    let bal_end = get_balance(&ctx, &user1.pubkey()).await.unwrap();

    let rent = ctx.banks_client.get_rent().await.unwrap();
    let min_rent_exempt = rent.minimum_balance(CLAIM_STATUS_LEN);

    assert_eq!(bal_end - bal_start, min_rent_exempt);
}

#[tokio::test]
async fn claim_happy_path() {
    let mut ctx = get_test().await;

    initialize(&ctx).await;

    let MerkleSetup {
        amount0,
        pre_balance0,
        tip_distribution_account,
        tree,
        user0,
        user1,
        validator_vote_account,
        ..
    } = setup_with_uploaded_merkle_root(&mut ctx, &jito_tip_distribution::id()).await;

    let index = 0;
    let proof = tree.get_proof(index);

    let (claim_status, _) =
        derive_claim_status_account_address(&user0.pubkey(), &tip_distribution_account);

    call_claim(
        &ctx,
        &tip_distribution_account,
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
        tip_distribution_account,
        tree,
        user0,
        user1,
        ..
    } = setup_with_uploaded_merkle_root(&mut ctx, &jito_tip_distribution::id()).await;

    let index = 0;
    let proof = tree.get_proof(index);

    let (claim_status, _) =
        derive_claim_status_account_address(&user0.pubkey(), &tip_distribution_account);

    let bad_authority = Keypair::new();

    let res = call_claim(
        &ctx,
        &tip_distribution_account,
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

    let (config_account_key, _) = derive_config_account_address();
    let (merkle_root_upload_config_key, _) = derive_merkle_root_upload_config_address();

    let original_authority = JITO_MERKLE_UPLOAD_AUTHORITY;
    let override_authority = Keypair::new();

    call_initialize_merkle_root_upload_config(
        &ctx,
        &config_account_key,
        &merkle_root_upload_config_key,
        &authority,
        &override_authority,
        &original_authority,
    )
    .await
    .unwrap();

    let setup = setup_init_tip_distribution_account(&ctx, &jito_tip_distribution::id()).await;

    call_init_tip_distribution_account(
        &ctx,
        &jito_tip_distribution::id(),
        setup.max_validator_commission_bps,
        &JITO_MERKLE_UPLOAD_AUTHORITY,
        &derive_config_account_address().0,
        &setup.validator_identity,
        &setup.validator_vote_account.pubkey(),
        &setup.tip_distribution_account,
        setup.bump,
    )
    .await
    .unwrap();

    call_migrate_tda_merkle_root_upload_authority(
        &ctx,
        &setup.tip_distribution_account,
        &merkle_root_upload_config_key,
    )
    .await
    .unwrap();

    let tda =
        get_deserialized_account::<TipDistributionAccount>(&ctx, &setup.tip_distribution_account)
            .await
            .unwrap()
            .unwrap();

    assert_eq!(
        tda.merkle_root_upload_authority,
        override_authority.pubkey()
    );
}

#[tokio::test]
async fn migrate_tda_merkle_root_upload_authority_should_error_if_tda_not_jito_authority() {
    let ctx = get_test().await;

    let authority = initialize(&ctx).await;

    let (config_account_key, _) = derive_config_account_address();
    let (merkle_root_upload_config_key, _) = derive_merkle_root_upload_config_address();

    let original_authority = JITO_MERKLE_UPLOAD_AUTHORITY;
    let override_authority = Keypair::new();

    call_initialize_merkle_root_upload_config(
        &ctx,
        &config_account_key,
        &merkle_root_upload_config_key,
        &authority,
        &override_authority,
        &original_authority,
    )
    .await
    .unwrap();

    let setup = setup_init_tip_distribution_account(&ctx, &jito_tip_distribution::id()).await;

    call_init_tip_distribution_account(
        &ctx,
        &jito_tip_distribution::id(),
        setup.max_validator_commission_bps,
        &setup.validator_vote_account.pubkey(),
        &derive_config_account_address().0,
        &setup.validator_identity,
        &setup.validator_vote_account.pubkey(),
        &setup.tip_distribution_account,
        setup.bump,
    )
    .await
    .unwrap();

    let res = call_migrate_tda_merkle_root_upload_authority(
        &ctx,
        &setup.tip_distribution_account,
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
async fn migrate_tda_merkle_root_upload_authority_should_error_if_merkle_root_is_already_uploaded()
{
    let mut ctx = get_test().await;

    let authority = initialize(&ctx).await;

    let (config_account_key, _) = derive_config_account_address();
    let (merkle_root_upload_config_key, _) = derive_merkle_root_upload_config_address();

    let original_authority = JITO_MERKLE_UPLOAD_AUTHORITY;
    let override_authority = Keypair::new();

    call_initialize_merkle_root_upload_config(
        &ctx,
        &config_account_key,
        &merkle_root_upload_config_key,
        &authority,
        &override_authority,
        &original_authority,
    )
    .await
    .unwrap();

    let MerkleSetup {
        tip_distribution_account,
        ..
    } = setup_with_uploaded_merkle_root(&mut ctx, &jito_tip_distribution::id()).await;

    let res = call_migrate_tda_merkle_root_upload_authority(
        &ctx,
        &tip_distribution_account,
        &merkle_root_upload_config_key,
    )
    .await;

    let err = res.unwrap_err();
    assert_eq!(
        err.unwrap(),
        TransactionError::InstructionError(0, InstructionError::Custom(6015))
    );
}
