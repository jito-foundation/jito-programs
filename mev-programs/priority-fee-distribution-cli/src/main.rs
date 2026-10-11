use std::{path::PathBuf, str::FromStr};

use anchor_lang::{system_program, AccountDeserialize, InstructionData, ToAccountMetas};
use bs58;
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
        default_value = "Priority6weCZ5HwDn29NxLFpb7TDp2iLZ6XKc5e8d3"
    )]
    program_id: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get the merkle root upload config account information
    GetMerkleRootUploadConfig,

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

    /// Initialize the priority fee distribution config account
    Initialize {
        /// Path to the authority keypair file
        #[arg(long)]
        authority_keypair_path: String,

        /// Authority pubkey
        #[arg(long)]
        authority: String,

        /// Expired funds account pubkey
        #[arg(long)]
        expired_funds_account: String,

        /// Max validator commission BPS
        #[arg(long)]
        max_validator_commission_bps: u16,

        /// Number of epochs valid
        #[arg(long)]
        num_epochs_valid: u64,
    },

    /// Initialize the merkle root upload config account
    InitializeMerkleRootUploadConfig {
        /// Path to the authority keypair file
        #[arg(long)]
        authority_keypair_path: String,

        /// Config account pubkey
        #[arg(long)]
        config: String,

        /// Authority pubkey
        #[arg(long)]
        authority: String,

        /// Payer pubkey
        #[arg(long)]
        payer: String,
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

    /// Update the merkle root upload config account
    UpdateMerkleRootUploadConfig {
        /// Authority pubkey
        #[arg(long)]
        authority: String,

        /// Original authority pubkey
        #[arg(long)]
        original_authority: String,
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
                authority: Pubkey::from_str(&authority)?,
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

        Commands::Initialize {
            authority_keypair_path,
            authority,
            expired_funds_account,
            max_validator_commission_bps,
            num_epochs_valid,
        } => {
            let authority_keypair = read_keypair_file(authority_keypair_path)
                .expect("Failed to read authority keypair file");
            let (config_pda, bump) = derive_config_account_address(&program_id);
            println!("Config Account Address: {}", config_pda);

            let authority_pubkey = Pubkey::from_str(&authority)?;
            let expired_funds_account_pubkey = Pubkey::from_str(&expired_funds_account)?;

            let instruction = Instruction {
                program_id,
                data: jito_priority_fee_distribution::instruction::Initialize {
                    authority: authority_pubkey,
                    expired_funds_account: expired_funds_account_pubkey,
                    num_epochs_valid,
                    max_validator_commission_bps,
                    bump,
                }
                .data(),
                accounts: jito_priority_fee_distribution::accounts::Initialize {
                    initializer: authority_pubkey,
                    config: config_pda,
                    system_program: solana_sdk::system_program::ID,
                }
                .to_account_metas(Some(true)),
            };

            let mut transaction =
                solana_sdk::transaction::Transaction::new_with_payer(&[instruction], None);
            transaction.sign(&[&authority_keypair], client.get_latest_blockhash()?);
            let signature = client.send_and_confirm_transaction_with_spinner(&transaction)?;
            println!("Transaction Signature: {}", signature);
        }

        Commands::InitializeMerkleRootUploadConfig {
            authority_keypair_path,
            config,
            authority,
            payer,
        } => {
            let authority_keypair = read_keypair_file(authority_keypair_path)
                .expect("Failed to read authority keypair file");
            let config_pubkey = Pubkey::from_str(&config)?;
            let authority_pubkey = Pubkey::from_str(&authority)?;
            let payer_pubkey = Pubkey::from_str(&payer)?;
            let (merkle_root_upload_config, _) =
                Pubkey::find_program_address(&[b"ROOT_UPLOAD_CONFIG"], &program_id);

            let instruction = Instruction {
                program_id,
                data:
                    jito_priority_fee_distribution::instruction::InitializeMerkleRootUploadConfig {
                        original_authority: authority_pubkey,
                        authority: authority_pubkey,
                    }
                    .data(),
                accounts:
                    jito_priority_fee_distribution::accounts::InitializeMerkleRootUploadConfig {
                        config: config_pubkey,
                        authority: authority_pubkey,
                        payer: payer_pubkey,
                        merkle_root_upload_config,
                        system_program: solana_sdk::system_program::ID,
                    }
                    .to_account_metas(None),
            };
            let mut transaction =
                solana_sdk::transaction::Transaction::new_with_payer(&[instruction], None);
            transaction.sign(&[&authority_keypair], client.get_latest_blockhash()?);
            let signature = client.send_and_confirm_transaction_with_spinner_and_config(
                &transaction,
                solana_sdk::commitment_config::CommitmentConfig::confirmed(),
                solana_client::rpc_config::RpcSendTransactionConfig {
                    skip_preflight: true,
                    ..Default::default()
                },
            )?;
            println!("Transaction Signature: {}", signature);
        }

        Commands::GetMerkleRootUploadConfig => {
            let (merkle_root_upload_config, _) =
                Pubkey::find_program_address(&[b"ROOT_UPLOAD_CONFIG"], &program_id);
            println!(
                "Merkle Root Upload Config Account Address: {}",
                merkle_root_upload_config
            );

            let account_data = client.get_account(&merkle_root_upload_config)?.data;
            let config: jito_priority_fee_distribution::state::MerkleRootUploadConfig =
                jito_priority_fee_distribution::state::MerkleRootUploadConfig::try_deserialize(
                    &mut account_data.as_slice(),
                )?;

            println!("Merkle Root Upload Config Account Data:");
            println!("  Original Authority: {}", config.original_upload_authority);
            println!("  Override Authority: {}", config.override_authority);
            println!("  Bump: {}", config.bump);
        }

        Commands::UpdateMerkleRootUploadConfig {
            authority,
            original_authority,
        } => {
            let authority_pubkey = Pubkey::from_str(&authority)?;
            let original_authority_pubkey = Pubkey::from_str(&original_authority)?;

            let (config_pda, _) = derive_config_account_address(&program_id);
            let (merkle_root_upload_config, _) =
                Pubkey::find_program_address(&[b"ROOT_UPLOAD_CONFIG"], &program_id);

            let instruction = Instruction {
                program_id,
                data: jito_priority_fee_distribution::instruction::UpdateMerkleRootUploadConfig {
                    authority: authority_pubkey,
                    original_authority: original_authority_pubkey,
                }
                .data(),
                accounts: jito_priority_fee_distribution::accounts::UpdateMerkleRootUploadConfig {
                    config: config_pda,
                    merkle_root_upload_config,
                    authority: authority_pubkey,
                    system_program: solana_sdk::system_program::ID,
                }
                .to_account_metas(None),
            };

            let serialized_data = instruction.data;
            let base58_data = bs58::encode(serialized_data).into_string();
            println!("Base58 Serialized Data: {}", base58_data);

            println!("\nAccounts:");
            for (i, account_meta) in instruction.accounts.iter().enumerate() {
                let writable_status = if account_meta.is_writable {
                    "writable"
                } else {
                    "readonly"
                };
                let signer_status = if account_meta.is_signer {
                    "signer"
                } else {
                    "non-signer"
                };
                println!(
                    "  {}: {} ({}, {})",
                    i, account_meta.pubkey, writable_status, signer_status
                );
            }
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
