use std::{rc::Rc, str::FromStr};

use anchor_lang::AccountDeserialize;
use clap::{Parser, Subcommand};
use jito_tip_payment::Config;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signature::Keypair};

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

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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

    // Use a dummy keypair since we're only reading data
    let payer = Rc::new(Keypair::new());
    let client = RpcClient::new(cli.rpc_url);

    match cli.command {
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
