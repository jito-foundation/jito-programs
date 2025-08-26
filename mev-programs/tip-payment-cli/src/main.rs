use std::str::FromStr;

use anchor_lang::{system_program, AccountDeserialize, InstructionData, ToAccountMetas};
use clap::{Parser, Subcommand};
use jito_tip_payment::{
    Config, InitBumps, CONFIG_ACCOUNT_SEED, TIP_ACCOUNT_SEED_0, TIP_ACCOUNT_SEED_1,
    TIP_ACCOUNT_SEED_2, TIP_ACCOUNT_SEED_3, TIP_ACCOUNT_SEED_4, TIP_ACCOUNT_SEED_5,
    TIP_ACCOUNT_SEED_6, TIP_ACCOUNT_SEED_7,
};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    instruction::Instruction, pubkey::Pubkey, signature::read_keypair_file, signer::Signer,
    transaction::Transaction,
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// RPC URL for the Solana cluster
    #[arg(short, long, default_value = "http://localhost:8899")]
    rpc_url: String,

    /// Tip Payment program ID
    #[arg(
        short,
        long,
        default_value = "T1pyyaTNZsKv2WcRAB8oVnk93mLJw2XzjtVYqCsaHqt"
    )]
    program_id: String,

    #[arg(short, long)]
    keypair_path: String,

    #[command(subcommand)]
    command: Commands,
}

#[allow(clippy::enum_variant_names)]
#[derive(Subcommand)]
enum Commands {
    /// Initialize the config account information
    InitConfig,

    /// Get the config account information
    GetConfig,

    /// Get all tip payment accounts information
    GetAllTipAccounts,

    /// Get a specific tip payment account
    GetTipAccount {
        /// Index of the tip account (0-7)
        #[arg(value_parser = clap::value_parser!(u8).range(0..8))]
        index: u8,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let program_id = Pubkey::from_str(&cli.program_id)?;
    let client = RpcClient::new(cli.rpc_url);

    let keypair = read_keypair_file(cli.keypair_path).expect("Failed to read keypair");

    match cli.command {
        Commands::InitConfig => {
            let (config_pubkey, config_bump) =
                Pubkey::find_program_address(&[CONFIG_ACCOUNT_SEED], &program_id);
            let (tip_payment_account_0_pubkey, tip_payment_account_0_bump) =
                Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_0], &program_id);
            let (tip_payment_account_1_pubkey, tip_payment_account_1_bump) =
                Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_1], &program_id);
            let (tip_payment_account_2_pubkey, tip_payment_account_2_bump) =
                Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_2], &program_id);
            let (tip_payment_account_3_pubkey, tip_payment_account_3_bump) =
                Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_3], &program_id);
            let (tip_payment_account_4_pubkey, tip_payment_account_4_bump) =
                Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_4], &program_id);
            let (tip_payment_account_5_pubkey, tip_payment_account_5_bump) =
                Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_5], &program_id);
            let (tip_payment_account_6_pubkey, tip_payment_account_6_bump) =
                Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_6], &program_id);
            let (tip_payment_account_7_pubkey, tip_payment_account_7_bump) =
                Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_7], &program_id);

            let ix = Instruction {
                program_id,
                data: jito_tip_payment::instruction::Initialize {
                    _bumps: InitBumps {
                        config: config_bump,
                        tip_payment_account_0: tip_payment_account_0_bump,
                        tip_payment_account_1: tip_payment_account_1_bump,
                        tip_payment_account_2: tip_payment_account_2_bump,
                        tip_payment_account_3: tip_payment_account_3_bump,
                        tip_payment_account_4: tip_payment_account_4_bump,
                        tip_payment_account_5: tip_payment_account_5_bump,
                        tip_payment_account_6: tip_payment_account_6_bump,
                        tip_payment_account_7: tip_payment_account_7_bump,
                    },
                }
                .data(),
                accounts: jito_tip_payment::accounts::Initialize {
                    config: config_pubkey,
                    tip_payment_account_0: tip_payment_account_0_pubkey,
                    tip_payment_account_1: tip_payment_account_1_pubkey,
                    tip_payment_account_2: tip_payment_account_2_pubkey,
                    tip_payment_account_3: tip_payment_account_3_pubkey,
                    tip_payment_account_4: tip_payment_account_4_pubkey,
                    tip_payment_account_5: tip_payment_account_5_pubkey,
                    tip_payment_account_6: tip_payment_account_6_pubkey,
                    tip_payment_account_7: tip_payment_account_7_pubkey,
                    payer: keypair.pubkey(),
                    system_program: system_program::ID,
                }
                .to_account_metas(None),
            };

            let blockhash = client.get_latest_blockhash().unwrap();
            let tx = Transaction::new_signed_with_payer(
                &[ix],
                Some(&keypair.pubkey()),
                &[keypair],
                blockhash,
            );

            client.send_transaction(&tx).unwrap();
        }
        Commands::GetConfig => {
            let config_pda =
                Pubkey::find_program_address(&[jito_tip_payment::CONFIG_ACCOUNT_SEED], &program_id)
                    .0;
            let config_data = client.get_account(&config_pda)?.data;
            let config: Config = Config::try_deserialize(&mut config_data.as_slice())?;
            println!("Config Account:");
            println!("  Tip Receiver: {}", config.tip_receiver);
            println!("  Block Builder: {}", config.block_builder);
            println!(
                "  Block Builder Commission %: {}",
                config.block_builder_commission_pct
            );
            println!("  Bumps:");
            println!("    Config: {}", config.bumps.config);
            println!("    Tip Account 0: {}", config.bumps.tip_payment_account_0);
            println!("    Tip Account 1: {}", config.bumps.tip_payment_account_1);
            println!("    Tip Account 2: {}", config.bumps.tip_payment_account_2);
            println!("    Tip Account 3: {}", config.bumps.tip_payment_account_3);
            println!("    Tip Account 4: {}", config.bumps.tip_payment_account_4);
            println!("    Tip Account 5: {}", config.bumps.tip_payment_account_5);
            println!("    Tip Account 6: {}", config.bumps.tip_payment_account_6);
            println!("    Tip Account 7: {}", config.bumps.tip_payment_account_7);
        }
        Commands::GetAllTipAccounts => {
            let tip_account_seeds = [
                jito_tip_payment::TIP_ACCOUNT_SEED_0,
                jito_tip_payment::TIP_ACCOUNT_SEED_1,
                jito_tip_payment::TIP_ACCOUNT_SEED_2,
                jito_tip_payment::TIP_ACCOUNT_SEED_3,
                jito_tip_payment::TIP_ACCOUNT_SEED_4,
                jito_tip_payment::TIP_ACCOUNT_SEED_5,
                jito_tip_payment::TIP_ACCOUNT_SEED_6,
                jito_tip_payment::TIP_ACCOUNT_SEED_7,
            ];

            for (i, seed) in tip_account_seeds.iter().enumerate() {
                let tip_pda = Pubkey::find_program_address(&[seed], &program_id).0;
                let lamports = client.get_account(&tip_pda)?.lamports;

                println!("Tip Payment Account {}:", i);
                println!("  Address: {}", tip_pda);
                println!("  Lamports: {}", lamports);
            }
        }
        Commands::GetTipAccount { index } => {
            let seed = match index {
                0 => jito_tip_payment::TIP_ACCOUNT_SEED_0,
                1 => jito_tip_payment::TIP_ACCOUNT_SEED_1,
                2 => jito_tip_payment::TIP_ACCOUNT_SEED_2,
                3 => jito_tip_payment::TIP_ACCOUNT_SEED_3,
                4 => jito_tip_payment::TIP_ACCOUNT_SEED_4,
                5 => jito_tip_payment::TIP_ACCOUNT_SEED_5,
                6 => jito_tip_payment::TIP_ACCOUNT_SEED_6,
                7 => jito_tip_payment::TIP_ACCOUNT_SEED_7,
                _ => unreachable!(),
            };

            let tip_pda = Pubkey::find_program_address(&[seed], &program_id).0;
            let lamports = client.get_account(&tip_pda)?.lamports;

            println!("Tip Payment Account {}:", index);
            println!("  Address: {}", tip_pda);
            println!("  Lamports: {}", lamports);
        }
    }

    Ok(())
}
