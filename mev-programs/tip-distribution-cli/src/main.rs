use std::str::FromStr;

use anchor_lang::{system_program, AccountDeserialize, InstructionData, ToAccountMetas};
use clap::{Parser, Subcommand};
use jito_tip_distribution::state::{ClaimStatus, Config, TipDistributionAccount};
use jito_tip_distribution_sdk::{
    derive_config_account_address, derive_merkle_root_upload_config_account_address,
    derive_tip_distribution_account_address,
    instruction::{update_config_ix, UpdateConfigAccounts, UpdateConfigArgs},
};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    instruction::Instruction, pubkey::Pubkey, signature::read_keypair_file, signer::Signer,
    transaction::Transaction,
};

fn parse_byte_array(s: &str) -> Result<[u8; 32], String> {
    let bytes: Result<Vec<u8>, _> = s.split(',').map(|x| x.trim().parse::<u8>()).collect();

    match bytes {
        Ok(vec) if vec.len() == 32 => {
            let mut array = [0u8; 32];
            array.copy_from_slice(&vec);
            Ok(array)
        }
        Ok(_) => Err("Must provide exactly 32 bytes".to_string()),
        Err(e) => Err(format!("Invalid byte: {}", e)),
    }
}

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

    #[arg(short, long)]
    keypair_path: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize the config account information
    InitConfig {
        /// Authority
        authority: Pubkey,

        /// Expired funds account
        expired_funds_account: Pubkey,

        /// Number of epochs is valid
        num_epochs_valid: u64,

        /// Max validator commission BPS
        max_validator_commission_bps: u16,
    },

    /// Get the config account information
    GetConfig,

    /// Get tip distribution account information for a specific validator and epoch
    InitializeTipDistributionAccount {
        /// Validator vote account pubkey
        #[arg(long)]
        vote_account: Pubkey,

        /// Merkle root upload authority
        #[arg(long)]
        merkle_root_upload_authority: Pubkey,

        /// Validator commission BPS
        #[arg(long)]
        validator_commission_bps: u16,
    },

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
    },

    /// Upload merkle root
    UploadMerkleRoot {
        #[arg(long)]
        vote_account: Pubkey,

        #[arg(long, value_parser = parse_byte_array)]
        root: [u8; 32],

        #[arg(long)]
        max_total_claim: u64,

        #[arg(long)]
        max_num_nodes: u64,
    },

    /// Initialize merkle root upload config
    InitializeMerkleRootUploadConfig {
        #[arg(long)]
        authority: Pubkey,

        #[arg(long)]
        original_authority: Pubkey,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let program_id = Pubkey::from_str(&cli.program_id)?;

    let client = RpcClient::new(cli.rpc_url);

    let keypair = read_keypair_file(cli.keypair_path).expect("Failed to read keypair");

    let (config_pda, config_bump) = derive_config_account_address(&program_id);
    let (merkle_root_upload_config_pda, _merkle_root_upload_config_bump) =
        derive_merkle_root_upload_config_account_address(&program_id);

    match cli.command {
        Commands::InitConfig {
            authority,
            expired_funds_account,
            num_epochs_valid,
            max_validator_commission_bps,
        } => {
            let ix = Instruction {
                program_id,
                data: jito_tip_distribution::instruction::Initialize {
                    authority,
                    expired_funds_account,
                    num_epochs_valid,
                    max_validator_commission_bps,
                    bump: config_bump,
                }
                .data(),
                accounts: jito_tip_distribution::accounts::Initialize {
                    config: config_pda,
                    system_program: system_program::ID,
                    initializer: keypair.pubkey(),
                }
                .to_account_metas(None),
            };

            let blockhash = client.get_latest_blockhash()?;
            let tx = Transaction::new_signed_with_payer(
                &[ix],
                Some(&keypair.pubkey()),
                &[keypair],
                blockhash,
            );

            client.send_transaction(&tx)?;
        }

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

        Commands::InitializeTipDistributionAccount {
            vote_account,
            merkle_root_upload_authority,
            validator_commission_bps,
        } => {
            let epoch = client.get_epoch_info()?.epoch;
            let (tip_distribution_pubkey, tip_distribution_bump) =
                derive_tip_distribution_account_address(&program_id, &vote_account, epoch);

            let ix = Instruction {
                program_id,
                data: jito_tip_distribution::instruction::InitializeTipDistributionAccount {
                    merkle_root_upload_authority,
                    validator_commission_bps,
                    bump: tip_distribution_bump,
                }
                .data(),
                accounts: jito_tip_distribution::accounts::InitializeTipDistributionAccount {
                    config: config_pda,
                    tip_distribution_account: tip_distribution_pubkey,
                    validator_vote_account: vote_account,
                    signer: keypair.pubkey(),
                    system_program: system_program::ID,
                }
                .to_account_metas(None),
            };

            let blockhash = client.get_latest_blockhash()?;
            let tx = Transaction::new_signed_with_payer(
                &[ix],
                Some(&keypair.pubkey()),
                &[keypair],
                blockhash,
            );

            client.send_transaction(&tx)?;
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
        } => {
            let authority_pubkey = Pubkey::from_str(&authority)?;
            let expired_funds_account_pubkey = Pubkey::from_str(&expired_funds_account)?;

            let config = Config {
                authority: authority_pubkey,
                expired_funds_account: expired_funds_account_pubkey,
                num_epochs_valid,
                max_validator_commission_bps,
                bump: config_bump,
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

            let blockhash = client.get_latest_blockhash()?;
            let tx = Transaction::new_signed_with_payer(
                &[instruction],
                Some(&keypair.pubkey()),
                &[keypair],
                blockhash,
            );

            client.send_transaction(&tx)?;

            // let serialized_data = instruction.data;
            // let base58_data = bs58::encode(serialized_data).into_string();
            // println!("Base58 Serialized Data: {}", base58_data);
        }
        Commands::UploadMerkleRoot {
            vote_account,
            root,
            max_total_claim,
            max_num_nodes,
        } => {
            let mut source: [u8; 32] = [0; 32];
            source.copy_from_slice(root.as_slice());

            let epoch = client.get_epoch_info()?.epoch;
            let (tip_distribution_pubkey, _tip_distribution_bump) =
                derive_tip_distribution_account_address(&program_id, &vote_account, epoch);

            let ix = Instruction {
                program_id,
                data: jito_tip_distribution::instruction::UploadMerkleRoot {
                    root: source,
                    max_total_claim,
                    max_num_nodes,
                }
                .data(),
                accounts: jito_tip_distribution::accounts::UploadMerkleRoot {
                    config: config_pda,
                    merkle_root_upload_authority: keypair.pubkey(),
                    tip_distribution_account: tip_distribution_pubkey,
                }
                .to_account_metas(None),
            };

            let blockhash = client.get_latest_blockhash()?;
            let tx = Transaction::new_signed_with_payer(
                &[ix],
                Some(&keypair.pubkey()),
                &[keypair],
                blockhash,
            );

            client.send_transaction(&tx)?;
        }
        Commands::InitializeMerkleRootUploadConfig {
            authority,
            original_authority,
        } => {
            let ix = Instruction {
                program_id,
                data: jito_tip_distribution::instruction::InitializeMerkleRootUploadConfig {
                    authority,
                    original_authority,
                }
                .data(),
                accounts: jito_tip_distribution::accounts::InitializeMerkleRootUploadConfig {
                    config: config_pda,
                    merkle_root_upload_config: merkle_root_upload_config_pda,
                    authority: keypair.pubkey(),
                    payer: keypair.pubkey(),
                    system_program: system_program::ID,
                }
                .to_account_metas(None),
            };

            let blockhash = client.get_latest_blockhash()?;
            let tx = Transaction::new_signed_with_payer(
                &[ix],
                Some(&keypair.pubkey()),
                &[keypair],
                blockhash,
            );

            client.send_transaction(&tx)?;
        }
    }

    Ok(())
}
