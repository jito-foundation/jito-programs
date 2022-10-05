mod generated_merkle_tree;

use std::{fs::File, io::BufReader, path::Path, str::FromStr};

use anchor_client::{solana_sdk::signature::Signer, Cluster};
use anchor_lang::AccountDeserialize;
use anchor_lang::{system_program::System, Id};
use clap::{value_t, App, Arg};
use generated_merkle_tree::GeneratedMerkleTreeCollection;
use solana_client::rpc_client::RpcClient;
use solana_program::{hash::Hash, instruction::Instruction, pubkey::Pubkey};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{read_keypair_file, Keypair},
    transaction::Transaction,
};
use tip_distribution::{
    sdk::instruction::{claim_ix, ClaimAccounts, ClaimArgs},
    state::ClaimStatus,
};
use tip_distribution::sdk::derive_config_account_address;
use solana_sdk::signature::Signature;
use std::time::{SystemTime, UNIX_EPOCH};
use csv::Writer;
use regex::Regex;

type Error = Box<dyn std::error::Error>;

pub struct RpcConfig {
    pub rpc_client: RpcClient,
    pub dry_run: bool,
    pub pid: Pubkey,
}

fn main() -> Result<(), Error> {
    let matches = App::new("claim")
        .arg({
            let arg = Arg::with_name("config_file")
                .short('C')
                .long("config")
                .value_name("PATH")
                .takes_value(true)
                .global(true)
                .help("Configuration file to use");
            if let Some(ref config_file) = *solana_cli_config::CONFIG_FILE {
                arg.default_value(config_file)
            } else {
                arg
            }
        })
        .arg(
            Arg::with_name("json_rpc_url")
                .long("url")
                .value_name("URL")
                .takes_value(true)
                .help("JSON RPC URL for the cluster.  Default from the configuration file."),
        )
        .arg(
            Arg::with_name("fee_payer")
                .long("fee-payer")
                .value_name("KEYPAIR")
                .takes_value(true)
                .help("Transaction fee payer account [default: cli config keypair]"),
        )
        .arg(
            Arg::with_name("dry_run")
                .long("dry-run")
                .takes_value(false)
                .global(true)
                .help("Simulate transaction instead of executing"),
        )
        .arg(
            Arg::with_name("merkle_tree")
                .long("merkle-tree")
                .takes_value(true)
                .global(true)
                .help("Filepath of merkle tree json"),
        )
        .arg(
            Arg::with_name("tip_distribution_pid")
                .short('t')
                .long("tip-distribution-pid")
                .takes_value(true)
                .help("Tip distribution account program id"),
        )
        .arg(
            Arg::with_name("log_dir")
                .long("log-dir")
                .takes_value(true)
                .help("Directory to log claim statuses")
        )
        .get_matches();

    let cli_config = if let Some(config_file) = matches.value_of("config_file") {
        solana_cli_config::Config::load(config_file).unwrap_or_default()
    } else {
        solana_cli_config::Config::default()
    };
    let json_rpc_url = value_t!(matches, "json_rpc_url", String)
        .unwrap_or_else(|_| cli_config.json_rpc_url.clone());
    let url = Cluster::from_str(json_rpc_url.as_str()).unwrap();

    let rpc_client = RpcClient::new_with_commitment(url, CommitmentConfig::confirmed());

    let fee_payer_path = if let Some(fee_payer) = matches.value_of("fee_payer") {
        fee_payer
    } else {
        cli_config.keypair_path.as_str()
    };

    let dry_run = matches.is_present("dry_run");
    let fee_payer = read_keypair_file(fee_payer_path).unwrap();

    let pid = value_t!(matches, "tip_distribution_pid", Pubkey)
        .expect("missing or invalid tip distribution pid!");

    let merkle_tree_path =
        value_t!(matches, "merkle_tree", String).expect("merkle tree path not found!");
    let merkle_tree = load_merkle_tree(&merkle_tree_path)?;

    let log_dir = matches.value_of("log_dir");

    let rpc_config = RpcConfig {
        rpc_client,
        dry_run,
        pid,
    };

    let results = command_claim_all(&rpc_config, &fee_payer, &merkle_tree);

    // if we made successful claim tx this cycle
    // write out the file with timestamp
    if log_dir.is_some() && !results.is_empty() {
        // 9-10 digits to cover next 100 years :D
        let re = Regex::new("[0-9]{9, 10}").unwrap();
        let slot = &re.captures(&merkle_tree_path).unwrap()[0];
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let filename = format!("{}claim-{}-{}.csv", log_dir.unwrap(), slot, now);
        if let Err(e) = write_csv(results, filename) {
            println!("error writing csv: {:?}", e);
        }
    }
    Ok(())
}

fn write_csv(results: Vec<(ClaimStatus, Signature)>, filename: String) -> Result<(), Error> {
    let mut wtr = Writer::from_path(filename)?;
    wtr.write_record(&["Claimant", "Amount Claimed", "Claim Tx Signature"])?;
    for result in results {
        let claimant = format!("{}", result.0.claimant);
        let amount = format!("{}", result.0.amount);
        let sig = format!("{}", result.1);
        wtr.write_record(&[claimant, amount, sig])?;
    }
    wtr.flush()?;
    Ok(())
}

/// runs workflow to claim all MEV rewards given a Generated merkle tree collection
fn command_claim_all(
    rpc_config: &RpcConfig,
    payer: &Keypair,
    merkle_tree: &GeneratedMerkleTreeCollection,
) -> Vec<(ClaimStatus, Signature)> {
    let mut total_claims = 0;
    let mut successful_claims = 0;
    let mut previously_claimed = 0;
    let mut results: Vec<(ClaimStatus, Signature)> = vec![];
    for tree in &merkle_tree.generated_merkle_trees {
        let tip_distribution_account = &tree.tip_distribution_account;
        for node in &tree.tree_nodes {
            let claimant = node.claimant;

            let claim_seeds = [
                ClaimStatus::SEED,
                claimant.as_ref(), // ordering matters here
                tip_distribution_account.as_ref(),
            ];

            let (claim_status, claim_bump) =
                Pubkey::find_program_address(&claim_seeds, &rpc_config.pid);

            total_claims += 1;
            if let Some(claim) = query_claim_status(
                rpc_config,
                &claim_status,
            ) {
                   if claim.is_claimed {
                       previously_claimed += 1;
                       continue;
                   }
            }

            let claim_args = ClaimArgs {
                proof: node.clone().proof.unwrap(),
                amount: node.clone().amount,
                bump: claim_bump,
            };

            let claim_accounts = ClaimAccounts {
                config: derive_config_account_address(&rpc_config.pid).0,
                tip_distribution_account: *tip_distribution_account,
                claim_status,
                claimant,
                payer: payer.pubkey(),
                system_program: System::id(),
            };

            let ix = claim_ix(rpc_config.pid, claim_args, claim_accounts);
            match send_transaction(rpc_config, &[ix.clone()], payer) {
                Ok((_, signature)) => {
                    match query_claim_status(
                        rpc_config,
                        &claim_status,
                    ) {
                        Some(claim_status) => {
                            if claim_status.is_claimed {
                                successful_claims += 1;
                            }
                            results.push((claim_status, signature));
                        }
                        None => {
                            println!("error getting claim_status for tx sig: {:?}", signature);
                        }
                    }
                }
                Err(e) => {
                    println!("error sending transaction: {:#?}, skipping", e);
                }
            }
        }
    }
    println!("Total Claims: {}, Previously Claimed: {}, Claimed: {}, Claim Errors: {}",
             total_claims,
             previously_claimed,
             successful_claims,
            total_claims - previously_claimed - successful_claims);
    results
}

fn load_merkle_tree<P: AsRef<Path>>(path: P) -> Result<GeneratedMerkleTreeCollection, Error> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let merkle_tree = serde_json::from_reader(reader)?;
    Ok(merkle_tree)
}
fn get_latest_blockhash(rpc_client: &RpcClient) -> Result<Hash, Error> {
    Ok(rpc_client
        .get_latest_blockhash_with_commitment(CommitmentConfig::confirmed())?
        .0)
}

fn query_claim_status(
    rpc_config: &RpcConfig,
    claim_status: &Pubkey,
) -> Option<ClaimStatus> {
    if let Ok(account_resp) = rpc_config.rpc_client
        .get_account_with_commitment(claim_status, CommitmentConfig::processed())
    {
            if let Some(account) = account_resp.value {
                let mut data: &[u8] = &account.data;
                if let Ok(claim_status) = ClaimStatus::try_deserialize(&mut data) {
                    return Some(claim_status)
                }
            }
    }
    None
}

/// Sends transaction payload, optionally simulating only
fn send_transaction(
    rpc_config: &RpcConfig,
    instructions: &[Instruction],
    fee_payer: &Keypair,
) -> Result<(Transaction, Signature), Error> {
    let recent_blockhash = get_latest_blockhash(&rpc_config.rpc_client)?;
    let transaction = Transaction::new_signed_with_payer(
        instructions,
        Some(&fee_payer.pubkey()),
        &[fee_payer],
        recent_blockhash,
    );

    let mut signature = Signature::default();
    if rpc_config.dry_run {
        let result = rpc_config.rpc_client.simulate_transaction(&transaction)?;
        println!("Simulate result: {:?}", result);
    } else {
        signature = rpc_config
            .rpc_client
            .send_and_confirm_transaction_with_spinner(&transaction)?;
    }
    Ok((transaction, signature))
}
