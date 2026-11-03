use anchor_lang::{InstructionData, ToAccountMetas};
use jito_tip_distribution::{
    state::{Config, TipDistributionAccount},
    ErrorCode,
};
use solana_program_test::{ProgramTest, ProgramTestContext};
use solana_sdk::{
    account::Account,
    clock::Clock,
    instruction::{Instruction, InstructionError},
    pubkey,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    signer::keypair::keypair_from_seed,
    system_program,
    transaction::{Transaction, TransactionError},
};

/// CI runs this against the `mainnet` build (`cargo build-sbf --features mainnet`).
const PROGRAM_ID: Pubkey = pubkey!("4R3gSG8BpU4t19KYj8CfnbtRpnT8gtk4dvTHxVRwc2r7");

/// node = new identity (seed 2), authorized voters {700: old (seed 1), 702: new}.
const VOTE_ACCOUNT: &[u8] =
    include_bytes!("../../vote-state/tests/fixtures/vote-account-rotation.bin");

async fn send(
    ctx: &mut ProgramTestContext,
    ix: Instruction,
    signer: &Keypair,
) -> Result<u64, TransactionError> {
    let blockhash = ctx.get_new_latest_blockhash().await.unwrap();
    let tx =
        Transaction::new_signed_with_payer(&[ix], Some(&signer.pubkey()), &[signer], blockhash);
    let result = ctx
        .banks_client
        .process_transaction_with_metadata(tx)
        .await
        .unwrap();
    result
        .result
        .map(|()| result.metadata.unwrap().compute_units_consumed)
}

async fn init_tda(
    ctx: &mut ProgramTestContext,
    vote: &Pubkey,
    signer: &Keypair,
    epoch: u64,
) -> Result<u64, TransactionError> {
    let mut clock: Clock = ctx.banks_client.get_sysvar().await.unwrap();
    clock.epoch = epoch;
    ctx.set_sysvar(&clock);
    let (tda, bump) = Pubkey::find_program_address(
        &[
            TipDistributionAccount::SEED,
            vote.as_ref(),
            &epoch.to_le_bytes(),
        ],
        &PROGRAM_ID,
    );
    let ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: jito_tip_distribution::accounts::InitializeTipDistributionAccount {
            config: Pubkey::find_program_address(&[Config::SEED], &PROGRAM_ID).0,
            tip_distribution_account: tda,
            validator_vote_account: *vote,
            signer: signer.pubkey(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
        data: jito_tip_distribution::instruction::InitializeTipDistributionAccount {
            merkle_root_upload_authority: Pubkey::new_unique(),
            validator_commission_bps: 1000,
            bump,
        }
        .data(),
    };
    send(ctx, ix, signer).await
}

#[tokio::test]
async fn test_init_tip_distribution_account_during_identity_rotation() {
    let old = keypair_from_seed(&[1; 32]).unwrap();
    let new = keypair_from_seed(&[2; 32]).unwrap();
    let (other, vote) = (Keypair::new(), Pubkey::new_unique());

    let mut test = ProgramTest::new("jito_tip_distribution", PROGRAM_ID, None);
    test.add_account(
        vote,
        Account {
            lamports: 1_000_000_000,
            data: VOTE_ACCOUNT.to_vec(),
            owner: solana_sdk::vote::program::id(),
            ..Account::default()
        },
    );
    for key in [&old, &new, &other] {
        test.add_account(
            key.pubkey(),
            Account::new(1_000_000_000, 0, &system_program::id()),
        );
    }
    let mut ctx = test.start_with_context().await;

    let payer = ctx.payer.insecure_clone();
    let (config, bump) = Pubkey::find_program_address(&[Config::SEED], &PROGRAM_ID);
    let ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: jito_tip_distribution::accounts::Initialize {
            config,
            system_program: system_program::id(),
            initializer: payer.pubkey(),
        }
        .to_account_metas(None),
        data: jito_tip_distribution::instruction::Initialize {
            authority: payer.pubkey(),
            expired_funds_account: payer.pubkey(),
            num_epochs_valid: 10,
            max_validator_commission_bps: 10_000,
            bump,
        }
        .data(),
    };
    send(&mut ctx, ix, &payer).await.unwrap();

    let unauthorized = Err(TransactionError::InstructionError(
        0,
        InstructionError::Custom(6000 + ErrorCode::Unauthorized as u32),
    ));
    // 701: node = new, voter = old. The old identity is still the scheduled leader.
    assert_eq!(init_tda(&mut ctx, &vote, &other, 701).await, unauthorized);
    let voter_cu = init_tda(&mut ctx, &vote, &old, 701).await.unwrap();
    // 702: voter = new, previous epoch's voter = old.
    init_tda(&mut ctx, &vote, &old, 702).await.unwrap();
    // 703: old is no longer a recent voter. The node identity always works.
    assert_eq!(init_tda(&mut ctx, &vote, &old, 703).await, unauthorized);
    let node_cu = init_tda(&mut ctx, &vote, &new, 703).await.unwrap();
    println!("compute units: node identity {node_cu}, authorized voter {voter_cu}");
}
