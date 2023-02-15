use std::{path::PathBuf, str::FromStr};

use anchor_client::{
    anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas},
    solana_sdk::signature::read_keypair_file,
};
use clap::Parser;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    instruction::Instruction, pubkey::Pubkey, signature::Signer, system_program,
    transaction::Transaction,
};
use tip_distribution::{accounts, instruction, state::Config};

#[derive(Parser, Debug)]
struct Args {
    #[clap(long, env)]
    rpc_url: String,

    #[clap(long, env)]
    tip_distribution_program_id: String,

    #[clap(long, env)]
    keypair_path: PathBuf,
}

fn main() {
    let args: Args = Args::parse();
    println!("args: {:?}", args);
    let payer = read_keypair_file(args.keypair_path).expect("Keypair required");

    let rpc_client = RpcClient::new(args.rpc_url);
    let tip_distribution_program_id =
        Pubkey::from_str(&args.tip_distribution_program_id).expect("valid program id");
    let (config_account_pubkey, config_account_bump) =
        Pubkey::find_program_address(&[b"CONFIG_ACCOUNT"], &tip_distribution_program_id);

    let config = Config::try_deserialize(
        &mut rpc_client
            .get_account(&config_account_pubkey)
            .expect("get account")
            .data
            .as_slice(),
    )
    .unwrap();

    let change_config_ix = Instruction {
        program_id: tip_distribution_program_id,
        data: instruction::UpdateConfig {
            new_config: Config {
                authority: config.authority,
                bump: config.bump,
                expired_funds_account: config.expired_funds_account,
                max_validator_commission_bps: 10_000,
                num_epochs_valid: config.num_epochs_valid,
            },
        }
        .data(),
        accounts: accounts::UpdateConfig {
            config: config_account_pubkey,
            authority: payer.pubkey(),
        }
        .to_account_metas(None),
    };

    let recent_blockhash = rpc_client.get_latest_blockhash().expect("latest blockhash");
    let transaction = Transaction::new_signed_with_payer(
        &[change_config_ix],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    let result = rpc_client.send_and_confirm_transaction(&transaction);
    assert!(result.is_ok(), "result: {:?}", result);
    println!("signature: {:?}", result.unwrap());

    let account = rpc_client
        .get_account(&config_account_pubkey)
        .expect("get account");
    let config =
        Config::try_deserialize(&mut account.data.as_slice()).expect("deserializes config");

    println!("config.authority: {:?}", config.authority);
    println!(
        "config.expired_funds_account: {:?}",
        config.expired_funds_account
    );
    println!("config.num_epochs_valid: {:?}", config.num_epochs_valid);
    println!(
        "config.max_validator_commission_bps: {:?}",
        config.max_validator_commission_bps
    );
    println!("config.bump: {:?}", config.bump);
}
