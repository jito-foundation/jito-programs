//! This binary is used to update the merkle root authority in case of mistaken
//! configuration or transfer of ownership.

use std::{path::PathBuf, str::FromStr, thread::sleep, time::Duration};

use clap::Parser;
use log::{error, info};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;
use solana_sdk::{
    signature::{read_keypair_file, Signature},
    signer::Signer,
    transaction::Transaction,
};
use tip_distribution::sdk::instruction::{
    set_merkle_root_upload_authority_ix, SetMerkleRootUploadAuthorityAccounts,
    SetMerkleRootUploadAuthorityArgs,
};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Path to fee payer keypair
    #[clap(long, env)]
    fee_payer: PathBuf,

    /// Path to validator vote account keypair
    #[clap(long, env)]
    validator_vote_account: PathBuf,

    /// Tip distribution account pubkey
    #[clap(long, env)]
    tip_distribution_account: String,

    /// New merkle root upload authority to use
    #[clap(long, env)]
    new_authority: String,

    /// Tip distribution program
    #[clap(long, env)]
    program_id: String,

    /// The RPC to submit transactions
    #[clap(long, env)]
    rpc_url: String,
}

const DELAY: Duration = Duration::from_millis(500);
const MAX_RETRIES: usize = 5;

fn main() {
    env_logger::init();
    info!("Updating merkle root authority...");

    let args: Args = Args::parse();
    let program_id = Pubkey::from_str(&*args.program_id).unwrap();
    let new_authority = Pubkey::from_str(&*args.new_authority).unwrap();
    let tip_distribution_account = Pubkey::from_str(&*args.tip_distribution_account).unwrap();

    let validator_vote_account = read_keypair_file(&args.validator_vote_account)
        .expect("Failed to read validator vote keypair file.");
    let validator_vote_account_pubkey = validator_vote_account.pubkey();
    let fee_payer_kp =
        read_keypair_file(&args.fee_payer).expect("Failed to read fee payer keypair file.");
    let fee_payer_pubkey = fee_payer_kp.pubkey();

    let rpc_client = RpcClient::new(args.rpc_url);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let recent_blockhash = runtime.block_on(rpc_client.get_latest_blockhash()).unwrap();

    let ix = set_merkle_root_upload_authority_ix(
        program_id,
        SetMerkleRootUploadAuthorityArgs {
            new_merkle_root_upload_authority: new_authority,
        },
        SetMerkleRootUploadAuthorityAccounts {
            tip_distribution_account,
            signer: validator_vote_account_pubkey,
        },
    );

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&fee_payer_pubkey),
        &[&validator_vote_account, &fee_payer_kp],
        recent_blockhash,
    );

    runtime.spawn(async move {
        if let Err(e) = send_transaction_with_retry(rpc_client, &tx, DELAY, MAX_RETRIES).await {
            error!(
                "error sending transaction [signature={}, error={}]",
                tx.signatures[0], e
            );
        } else {
            info!(
                "successfully sent transaction: [signature={}]",
                tx.signatures[0]
            );
        }
    });
}

async fn send_transaction_with_retry(
    rpc_client: RpcClient,
    tx: &Transaction,
    delay: Duration,
    max_retries: usize,
) -> solana_client::client_error::Result<Signature> {
    let mut retry_count: usize = 0;
    loop {
        match rpc_client.send_and_confirm_transaction(tx).await {
            Ok(sig) => {
                return Ok(sig);
            }
            Err(e) => {
                retry_count = retry_count.checked_add(1).unwrap();
                if retry_count == max_retries {
                    return Err(e);
                }
                sleep(delay);
            }
        }
    }
}
