use std::{path::PathBuf, str::FromStr};

use anchor_lang::{system_program, AccountDeserialize, InstructionData, ToAccountMetas};
use clap::{Parser, Subcommand};
use jito_priority_fee_distribution::state::{ClaimStatus, Config, PriorityFeeDistributionAccount};
use jito_priority_fee_distribution_sdk::{
    derive_config_account_address, derive_priority_fee_distribution_account_address,
};
use solana_client::{rpc_client::RpcClient, rpc_config::RpcSendTransactionConfig};
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction,
    instruction::Instruction,
    pubkey::Pubkey,
    signer::{keypair::read_keypair_file, Signer},
    transaction::Transaction,
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// RPC URL for the Solana cluster
    #[arg(short, long, default_value = "http://localhost:8899")]
    rpc_url: String,

    /// Priority Fee Distribution program ID
    #[arg(
        short,
        long,
        default_value = "5DdB5ZuSR97rqgVHtjb4t1uz1auFEa2xQ32aAxjsJLEC"
    )]
    program_id: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get the config account information
    GetConfig,

    /// Get priority fee distribution account information for a specific validator and epoch
    GetPriorityFeeDistributionAccount {
        /// Validator vote account pubkey
        #[arg(long)]
        vote_account: String,

        /// Epoch for the priority fee distribution account
        #[arg(long)]
        epoch: u64,
    },

    /// Get claim status for a specific validator, epoch and claimant
    GetClaimStatus {
        /// Validator vote account pubkey
        #[arg(long)]
        vote_account: String,

        /// Epoch for the priority fee distribution account
        #[arg(long)]
        epoch: u64,

        /// Claimant pubkey
        #[arg(long)]
        claimant: String,
    },

    /// Update the config account information
    UpdateConfig {
        /// Authority pubkey
        #[arg(long)]
        authority: String,

        /// Expired funds account pubkey
        #[arg(long)]
        expired_funds_account: String,

        /// Number of epochs valid
        #[arg(long)]
        num_epochs_valid: u64,

        /// Max validator commission BPS
        #[arg(long)]
        max_validator_commission_bps: u16,

        /// Bump
        #[arg(long)]
        bump: u8,

        /// The go_live_epoch for actual priority fee transfers (see
        ///  _transfer_priority_fee_tips_ instruction)
        #[arg(long)]
        go_live_epoch: u64,
    },

    TransferPriorityFeeTips {
        /// Path to Keypair that will make the transfer
        #[arg(long)]
        keypair_path: PathBuf,

        /// Validator vote account pubkey
        #[arg(long)]
        vote_account: String,

        /// Epoch for the priority fee distribution account
        #[arg(long)]
        epoch: u64,

        /// The amount of lamports to transfer from the Keypair to the
        /// PriorityFeeDistributionAccount
        #[arg(long)]
        lamports: u64,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let program_id = Pubkey::from_str(&cli.program_id)?;

    let client = RpcClient::new(cli.rpc_url);

    match cli.command {
        Commands::GetConfig => {
            let (config_pda, _) = derive_config_account_address(&program_id);
            println!("Config Account Address: {}", config_pda);

            let config_data = client.get_account(&config_pda)?.data;
            let config: Config = Config::try_deserialize(&mut config_data.as_slice())?;

            println!("Config Account Data:");
            println!("  Authority: {}", config.authority);
            println!("  Expired Funds Account: {}", config.expired_funds_account);
            println!("  Num Epochs Valid: {}", config.num_epochs_valid);
            println!(
                "  Max Validator Commission BPS: {}",
                config.max_validator_commission_bps
            );
            println!("  Go Live Epoch: {}", config.go_live_epoch);
            println!("  Bump: {}", config.bump);
        }

        Commands::GetPriorityFeeDistributionAccount {
            vote_account,
            epoch,
        } => {
            let vote_pubkey = Pubkey::from_str(&vote_account)?;
            let (priority_fee_dist_pda, _) =
                derive_priority_fee_distribution_account_address(&program_id, &vote_pubkey, epoch);
            println!(
                "Tip Distribution Account Address: {}",
                priority_fee_dist_pda
            );

            let account_data = client.get_account(&priority_fee_dist_pda)?.data;
            let priority_fee_dist: PriorityFeeDistributionAccount =
                PriorityFeeDistributionAccount::try_deserialize(&mut account_data.as_slice())?;

            println!("Priority Fee Distribution Account Data:");
            println!(
                "  Vote Account: {}",
                priority_fee_dist.validator_vote_account
            );
            println!(
                "  Merkle Root Upload Authority: {}",
                priority_fee_dist.merkle_root_upload_authority
            );
            println!("  Epoch Created At: {}", priority_fee_dist.epoch_created_at);
            println!(
                "  Validator Commission BPS: {}",
                priority_fee_dist.validator_commission_bps
            );
            println!("  Expires At: {}", priority_fee_dist.expires_at);
            println!(
                "  Total Lamports Transferred: {}",
                priority_fee_dist.total_lamports_transferred
            );
            println!("  Bump: {}", priority_fee_dist.bump);

            if let Some(merkle_root) = priority_fee_dist.merkle_root {
                println!("  Merkle Root:");
                println!("    Root: {:?}", merkle_root.root);
                println!("    Max Total Claim: {}", merkle_root.max_total_claim);
                println!("    Max Num Nodes: {}", merkle_root.max_num_nodes);
                println!(
                    "    Total Funds Claimed: {}",
                    merkle_root.total_funds_claimed
                );
                println!("    Num Nodes Claimed: {}", merkle_root.num_nodes_claimed);
            } else {
                println!("  Merkle Root: None");
            }
        }

        Commands::GetClaimStatus {
            vote_account,
            epoch,
            claimant,
        } => {
            let vote_pubkey = Pubkey::from_str(&vote_account)?;
            let claimant_pubkey = Pubkey::from_str(&claimant)?;

            // First get the priority fee distribution account address
            let (priority_fee_dist_pda, _) =
                derive_priority_fee_distribution_account_address(&program_id, &vote_pubkey, epoch);

            // Then derive claim status PDA using same seeds as in the program
            let (claim_status_pda, _) = Pubkey::find_program_address(
                &[
                    ClaimStatus::SEED,
                    claimant_pubkey.as_ref(),
                    priority_fee_dist_pda.as_ref(),
                ],
                &program_id,
            );
            println!("Claim Status Account Address: {}", claim_status_pda);

            let account_data = client.get_account(&claim_status_pda)?.data;
            let claim_status: ClaimStatus =
                ClaimStatus::try_deserialize(&mut account_data.as_slice())?;

            println!("Claim Status Data:");
            println!("  Expires At: {}", claim_status.expires_at);
        }

        Commands::UpdateConfig {
            authority,
            expired_funds_account,
            num_epochs_valid,
            max_validator_commission_bps,
            bump,
            go_live_epoch,
        } => {
            let authority_pubkey = Pubkey::from_str(&authority)?;
            let expired_funds_account_pubkey = Pubkey::from_str(&expired_funds_account)?;

            let config = Config {
                authority: authority_pubkey,
                expired_funds_account: expired_funds_account_pubkey,
                num_epochs_valid,
                max_validator_commission_bps,
                bump,
                go_live_epoch,
            };

            let (config_pda, _) = derive_config_account_address(&program_id);

            let instruction = Instruction {
                program_id,
                data: jito_priority_fee_distribution::instruction::UpdateConfig {
                    new_config: config,
                }
                .data(),
                accounts: jito_priority_fee_distribution::accounts::UpdateConfig {
                    config: config_pda,
                    authority: authority_pubkey,
                }
                .to_account_metas(None),
            };

            let serialized_data = instruction.data;
            let base58_data = bs58::encode(serialized_data).into_string();
            println!("Base58 Serialized Data: {}", base58_data);
        }
        Commands::TransferPriorityFeeTips {
            keypair_path,
            vote_account,
            epoch,
            lamports,
        } => {
            let keypair = read_keypair_file(&keypair_path).expect("Failed to read keypair file");

            let (config_pda, _) = derive_config_account_address(&program_id);
            let vote_pubkey = Pubkey::from_str(&vote_account)?;
            let (priority_fee_dist_pda, _) =
                derive_priority_fee_distribution_account_address(&program_id, &vote_pubkey, epoch);

            let instruction = Instruction {
                program_id,
                data: jito_priority_fee_distribution::instruction::TransferPriorityFeeTips {
                    lamports,
                }
                .data(),
                accounts: jito_priority_fee_distribution::accounts::TransferPriorityFeeTips {
                    config: config_pda,
                    priority_fee_distribution_account: priority_fee_dist_pda,
                    from: keypair.pubkey(),
                    system_program: system_program::ID,
                }
                .to_account_metas(None),
            };

            // tests show ~6,800 before go_live_epoch and ~6,400 after
            let compute_ix = ComputeBudgetInstruction::set_compute_unit_limit(8_000);

            let blockhash = client.get_latest_blockhash()?;

            let tx = Transaction::new_signed_with_payer(
                &[compute_ix, instruction],
                Some(&keypair.pubkey()),
                &vec![&keypair],
                blockhash,
            );
            let result = client.send_and_confirm_transaction_with_spinner_and_config(
                &tx,
                client.commitment(),
                RpcSendTransactionConfig::default(),
            )?;

            println!("TX Confirmed: {}", result);
        }
    }

    Ok(())
}
