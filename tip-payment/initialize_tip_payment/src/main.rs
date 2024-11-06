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

use tip_payment::{
Config, InitBumps, CONFIG_ACCOUNT_SEED, TIP_ACCOUNT_SEED_0,
TIP_ACCOUNT_SEED_1, TIP_ACCOUNT_SEED_2, TIP_ACCOUNT_SEED_3, TIP_ACCOUNT_SEED_4,
TIP_ACCOUNT_SEED_5, TIP_ACCOUNT_SEED_6, TIP_ACCOUNT_SEED_7,
};

#[derive(Parser, Debug)]
struct Args {
    #[clap(long, env)]
    rpc_url: String,

    #[clap(long, env)]
    tip_payment_program_id: String,

    #[clap(long, env)]
    keypair_path: PathBuf,
}

fn main() {
    let args: Args = Args::parse();
    println!("args: {:?}", args);
    let payer = read_keypair_file(args.keypair_path).expect("Keypair required");

    let rpc_client = RpcClient::new(args.rpc_url);
    let tip_payment_program_id =
        Pubkey::from_str(&args.tip_payment_program_id).expect("valid program id");
    let (config_account_pubkey, _config_account_bump) =
        Pubkey::find_program_address(&[b"CONFIG_ACCOUNT"], &tip_payment_program_id);

    // let config = Config::try_deserialize(
    //     &mut rpc_client
    //         .get_account(&config_account_pubkey)
    //         .expect("get account")
    //         .data
    //         .as_slice(),
    // )
    // .unwrap();

    let config_pda_bump =
        Pubkey::find_program_address(&[CONFIG_ACCOUNT_SEED], &tip_payment_program_id);
    let tip_pda_0 =
        Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_0], &tip_payment_program_id);
    let tip_pda_1 =
        Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_1], &tip_payment_program_id);
    let tip_pda_2 =
        Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_2], &tip_payment_program_id);
    let tip_pda_3 =
        Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_3], &tip_payment_program_id);
    let tip_pda_4 =
        Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_4], &tip_payment_program_id);
    let tip_pda_5 =
        Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_5], &tip_payment_program_id);
    let tip_pda_6 =
        Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_6], &tip_payment_program_id);
    let tip_pda_7 =
        Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_7], &tip_payment_program_id);



    let initialize_config_ix = Instruction {
        program_id: tip_payment_program_id,
        data: tip_payment::instruction::Initialize {
            _bumps: InitBumps {
                config: config_pda_bump.1,
                tip_payment_account_0: tip_pda_0.1,
                tip_payment_account_1: tip_pda_1.1,
                tip_payment_account_2: tip_pda_2.1,
                tip_payment_account_3: tip_pda_3.1,
                tip_payment_account_4: tip_pda_4.1,
                tip_payment_account_5: tip_pda_5.1,
                tip_payment_account_6: tip_pda_6.1,
                tip_payment_account_7: tip_pda_7.1,
            },
        }.data(),
        accounts: tip_payment::accounts::Initialize {
            config: config_pda_bump.0,
            tip_payment_account_0: tip_pda_0.0,
            tip_payment_account_1: tip_pda_1.0,
            tip_payment_account_2: tip_pda_2.0,
            tip_payment_account_3: tip_pda_3.0,
            tip_payment_account_4: tip_pda_4.0,
            tip_payment_account_5: tip_pda_5.0,
            tip_payment_account_6: tip_pda_6.0,
            tip_payment_account_7: tip_pda_7.0,
            system_program: system_program::id(),
            payer: payer.pubkey(),
        }.to_account_metas(None),
    };

    // let change_config_ix = Instruction {
    //     program_id: tip_payment_program_id,
    //     data: instruction::UpdateConfig {
    //         new_config: Config {
    //             authority: config.authority,
    //             bump: config.bump,
    //             expired_funds_account: config.expired_funds_account,
    //             max_validator_commission_bps: 10_000,
    //             num_epochs_valid: config.num_epochs_valid,
    //         },
    //     }
    //     .data(),
    //     accounts: accounts::UpdateConfig {
    //         config: config_account_pubkey,
    //         authority: payer.pubkey(),
    //     }
    //     .to_account_metas(None),
    // };

    let recent_blockhash = rpc_client.get_latest_blockhash().expect("latest blockhash");
    let transaction = Transaction::new_signed_with_payer(
        &[initialize_config_ix],
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

    println!("config.tip_receiver: {:?}", config.tip_receiver);
    println!("config.block_builder: {:?}", config.block_builder);
    println!(
        "config.block_builder_commission_pct: {:?}",
        config.block_builder_commission_pct
    );
    println!("config.bump0: {:?}", config.bumps.tip_payment_account_0);
    println!("config.bump1: {:?}", config.bumps.tip_payment_account_1);
    println!("config.bump2: {:?}", config.bumps.tip_payment_account_2);
    println!("config.bump3: {:?}", config.bumps.tip_payment_account_3);
    println!("config.bump4: {:?}", config.bumps.tip_payment_account_4);
    println!("config.bump5: {:?}", config.bumps.tip_payment_account_5);
    println!("config.bump6: {:?}", config.bumps.tip_payment_account_6);
    println!("config.bump7: {:?}", config.bumps.tip_payment_account_7);
}
