use std::str::FromStr;

use anchor_lang::AccountDeserialize;
use clap::{Parser, Subcommand};
use jito_tip_distribution::state::{ClaimStatus, Config, TipDistributionAccount};
use jito_tip_distribution_sdk::{
    derive_config_account_address, derive_tip_distribution_account_address,
    instruction::{update_config_ix, UpdateConfigAccounts, UpdateConfigArgs},
};
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// RPC URL for the Solana cluster
    #[arg(short, long, default_value = "http://localhost:8899")]
    rpc_url: String,

    /// Tip Distribution program ID
    #[arg(
        short,
        long,
        default_value = "4R3gSG8BpU4t19KYj8CfnbtRpnT8gtk4dvTHxVRwc2r7"
    )]
    program_id: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get the config account information
    GetConfig,

    /// Get tip distribution account information for a specific validator and epoch
    GetTipDistributionAccount {
        /// Validator vote account pubkey
        #[arg(long)]
        vote_account: String,

        /// Epoch for the tip distribution account
        #[arg(long)]
        epoch: u64,
    },

    /// Get claim status for a specific validator, epoch and claimant
    GetClaimStatus {
        /// Validator vote account pubkey
        #[arg(long)]
        vote_account: String,

        /// Epoch for the tip distribution account
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
            println!("  Bump: {}", config.bump);
        }

        Commands::GetTipDistributionAccount {
            vote_account,
            epoch,
        } => {
            let vote_pubkey = Pubkey::from_str(&vote_account)?;
            let (tip_dist_pda, _) =
                derive_tip_distribution_account_address(&program_id, &vote_pubkey, epoch);
            println!("Tip Distribution Account Address: {}", tip_dist_pda);

            let account_data = client.get_account(&tip_dist_pda)?.data;
            let tip_dist: TipDistributionAccount =
                TipDistributionAccount::try_deserialize(&mut account_data.as_slice())?;

            println!("Tip Distribution Account Data:");
            println!("  Vote Account: {}", tip_dist.validator_vote_account);
            println!(
                "  Merkle Root Upload Authority: {}",
                tip_dist.merkle_root_upload_authority
            );
            println!("  Epoch Created At: {}", tip_dist.epoch_created_at);
            println!(
                "  Validator Commission BPS: {}",
                tip_dist.validator_commission_bps
            );
            println!("  Expires At: {}", tip_dist.expires_at);
            println!("  Bump: {}", tip_dist.bump);

            if let Some(merkle_root) = tip_dist.merkle_root {
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

            // First get the tip distribution account address
            let (tip_dist_pda, _) =
                derive_tip_distribution_account_address(&program_id, &vote_pubkey, epoch);

            // Then derive claim status PDA using same seeds as in the program
            let (claim_status_pda, _) = Pubkey::find_program_address(
                &[
                    ClaimStatus::SEED,
                    claimant_pubkey.as_ref(),
                    tip_dist_pda.as_ref(),
                ],
                &program_id,
            );
            println!("Claim Status Account Address: {}", claim_status_pda);

            let account_data = client.get_account(&claim_status_pda)?.data;
            let claim_status: ClaimStatus =
                ClaimStatus::try_deserialize(&mut account_data.as_slice())?;

            println!("Claim Status Data:");
            println!("  Is Claimed: {}", claim_status.is_claimed);
            println!("  Claimant: {}", claim_status.claimant);
            println!("  Claim Status Payer: {}", claim_status.claim_status_payer);
            println!("  Slot Claimed At: {}", claim_status.slot_claimed_at);
            println!("  Amount: {}", claim_status.amount);
            println!("  Expires At: {}", claim_status.expires_at);
            println!("  Bump: {}", claim_status.bump);
        }

        Commands::UpdateConfig {
            authority,
            expired_funds_account,
            num_epochs_valid,
            max_validator_commission_bps,
            bump,
        } => {
            let authority_pubkey = Pubkey::from_str(&authority)?;
            let expired_funds_account_pubkey = Pubkey::from_str(&expired_funds_account)?;

            let config = Config {
                authority: authority_pubkey,
                expired_funds_account: expired_funds_account_pubkey,
                num_epochs_valid,
                max_validator_commission_bps,
                bump,
            };

            let accounts = UpdateConfigAccounts {
                config: Pubkey::default(),
                authority: authority_pubkey,
            };

            let instruction = update_config_ix(
                program_id,
                UpdateConfigArgs { new_config: config },
                accounts,
            );

            let serialized_data = instruction.data;
            let base58_data = bs58::encode(serialized_data).into_string();
            println!("Base58 Serialized Data: {}", base58_data);
        }
    }

    Ok(())
}
