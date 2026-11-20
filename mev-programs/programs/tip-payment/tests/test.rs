#[cfg(test)]
mod tests {
    use anchor_lang::{
        error::{ErrorCode, ERROR_CODE_OFFSET},
        solana_program::instruction::InstructionError,
        AccountDeserialize, AnchorSerialize, Discriminator, InstructionData, ToAccountMetas,
    };
    use jito_tip_payment::{
        Config, InitBumps, TipPaymentAccount, TipPaymentError, CONFIG_ACCOUNT_SEED,
        TIP_ACCOUNT_SEED_0, TIP_ACCOUNT_SEED_1, TIP_ACCOUNT_SEED_2, TIP_ACCOUNT_SEED_3,
        TIP_ACCOUNT_SEED_4, TIP_ACCOUNT_SEED_5, TIP_ACCOUNT_SEED_6, TIP_ACCOUNT_SEED_7,
        TIP_DISTRIBUTION_ACCOUNT_SEED, TIP_DISTRIBUTION_PROGRAM_ID,
    };
    use solana_program_test::{BanksClient, ProgramTest, ProgramTestContext};
    use solana_sdk::{
        account::{Account, ReadableAccount},
        bpf_loader_upgradeable,
        clock::Clock,
        commitment_config::CommitmentLevel,
        hash::Hash,
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        rent::Rent,
        reserved_account_keys::ReservedAccountKeys,
        signature::{Keypair, Signer},
        system_instruction::transfer,
        system_program,
        transaction::{Transaction, TransactionError},
    };

    async fn get_test(initial_accounts: &[(Pubkey, Account)]) -> ProgramTestContext {
        let mut test = ProgramTest::default();
        test.deactivate_feature(agave_feature_set::remove_accounts_executable_flag_checks::id());
        test.add_upgradeable_program_to_genesis("jito_tip_payment", &jito_tip_payment::id());
        test.add_upgradeable_program_to_genesis(
            "jito_tip_distribution",
            &jito_tip_distribution::id(),
        );

        for (pubkey, account) in initial_accounts {
            test.add_account(*pubkey, account.clone());
        }

        test.start_with_context().await
    }

    fn get_tip_pdas() -> Vec<(Pubkey, u8)> {
        let tip_pda_0 =
            Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_0], &jito_tip_payment::id());
        let tip_pda_1 =
            Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_1], &jito_tip_payment::id());
        let tip_pda_2 =
            Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_2], &jito_tip_payment::id());
        let tip_pda_3 =
            Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_3], &jito_tip_payment::id());
        let tip_pda_4 =
            Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_4], &jito_tip_payment::id());
        let tip_pda_5 =
            Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_5], &jito_tip_payment::id());
        let tip_pda_6 =
            Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_6], &jito_tip_payment::id());
        let tip_pda_7 =
            Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_7], &jito_tip_payment::id());
        vec![
            tip_pda_0, tip_pda_1, tip_pda_2, tip_pda_3, tip_pda_4, tip_pda_5, tip_pda_6, tip_pda_7,
        ]
    }

    fn get_config_pda() -> (Pubkey, u8) {
        Pubkey::find_program_address(&[CONFIG_ACCOUNT_SEED], &&jito_tip_payment::id())
    }

    /// Builds a vote-program-owned account whose embedded node_pubkey (bytes [4..36], per
    /// `VoteState::deserialize_node_pubkey`) is `node_pubkey`. The rest of the data is unused by
    /// `change_tip_receiver`'s validation.
    fn build_fake_vote_account(node_pubkey: &Pubkey) -> Account {
        let mut data = vec![0u8; 36];
        data[4..36].copy_from_slice(node_pubkey.as_ref());
        Account {
            lamports: Rent::default().minimum_balance(data.len()),
            data,
            owner: solana_sdk_ids::vote::id(),
            executable: false,
            rent_epoch: u64::MAX,
        }
    }

    /// Derives the TipDistributionAccount PDA for `vote_account` at `epoch`, using the same
    /// program id / seed `change_tip_receiver`'s validation uses internally.
    fn get_tip_distribution_pda(vote_account: &Pubkey, epoch: u64) -> Pubkey {
        Pubkey::find_program_address(
            &[
                TIP_DISTRIBUTION_ACCOUNT_SEED,
                vote_account.as_ref(),
                epoch.to_le_bytes().as_ref(),
            ],
            &TIP_DISTRIBUTION_PROGRAM_ID,
        )
        .0
    }

    async fn get_epoch(banks_client: &BanksClient) -> u64 {
        banks_client.get_sysvar::<Clock>().await.unwrap().epoch
    }

    async fn initialize_program(banks_client: &mut BanksClient, payer: &Keypair, blockhash: Hash) {
        let config_pda_bump = get_config_pda();
        let tip_pdas = get_tip_pdas();
        let init_ix = Instruction {
            program_id: jito_tip_payment::id(),
            data: jito_tip_payment::instruction::Initialize {
                _bumps: InitBumps {
                    config: config_pda_bump.1,
                    tip_payment_account_0: tip_pdas[0].1,
                    tip_payment_account_1: tip_pdas[1].1,
                    tip_payment_account_2: tip_pdas[2].1,
                    tip_payment_account_3: tip_pdas[3].1,
                    tip_payment_account_4: tip_pdas[4].1,
                    tip_payment_account_5: tip_pdas[5].1,
                    tip_payment_account_6: tip_pdas[6].1,
                    tip_payment_account_7: tip_pdas[7].1,
                },
            }
            .data(),
            accounts: jito_tip_payment::accounts::Initialize {
                config: config_pda_bump.0,
                tip_payment_account_0: tip_pdas[0].0,
                tip_payment_account_1: tip_pdas[1].0,
                tip_payment_account_2: tip_pdas[2].0,
                tip_payment_account_3: tip_pdas[3].0,
                tip_payment_account_4: tip_pdas[4].0,
                tip_payment_account_5: tip_pdas[5].0,
                tip_payment_account_6: tip_pdas[6].0,
                tip_payment_account_7: tip_pdas[7].0,
                system_program: system_program::id(),
                payer: payer.pubkey(),
            }
            .to_account_metas(None),
        };

        let tx = Transaction::new_signed_with_payer(
            &[init_ix],
            Some(&payer.pubkey()),
            &[&payer],
            blockhash,
        );
        banks_client
            .process_transaction_with_commitment(tx, CommitmentLevel::Processed)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_init_ok() {
        let ProgramTestContext {
            mut banks_client,
            last_blockhash,
            payer,
            ..
        } = get_test(&[]).await;

        initialize_program(&mut banks_client, &payer, last_blockhash).await;

        for (pda, _) in get_tip_pdas() {
            let account = banks_client
                .get_account_with_commitment(pda, CommitmentLevel::Processed)
                .await
                .unwrap()
                .unwrap();
            TipPaymentAccount::try_deserialize(&mut account.data.as_slice()).unwrap();
        }
    }

    #[tokio::test]
    async fn test_change_tip_receiver_different_tip_receiver_fails() {
        let ProgramTestContext {
            mut banks_client,
            last_blockhash,
            payer,
            ..
        } = get_test(&[]).await;

        initialize_program(&mut banks_client, &payer, last_blockhash).await;

        let config_pda = get_config_pda();
        let tip_pdas = get_tip_pdas();

        let change_tip_receiver_ix = Instruction {
            program_id: jito_tip_payment::id(),
            data: jito_tip_payment::instruction::ChangeTipReceiver {}.data(),
            accounts: jito_tip_payment::accounts::ChangeTipReceiver {
                config: config_pda.0,
                old_tip_receiver: Pubkey::new_unique(), // bad tip receiver
                new_tip_receiver: Pubkey::new_unique(),
                block_builder: payer.pubkey(),
                tip_payment_account_0: tip_pdas[0].0,
                tip_payment_account_1: tip_pdas[1].0,
                tip_payment_account_2: tip_pdas[2].0,
                tip_payment_account_3: tip_pdas[3].0,
                tip_payment_account_4: tip_pdas[4].0,
                tip_payment_account_5: tip_pdas[5].0,
                tip_payment_account_6: tip_pdas[6].0,
                tip_payment_account_7: tip_pdas[7].0,
                signer: payer.pubkey(),
                // placeholder: old_tip_receiver's constraint fails before this is ever read.
                validator_vote_account: Pubkey::new_unique(),
            }
            .to_account_metas(None),
        };
        let tx = Transaction::new_signed_with_payer(
            &[change_tip_receiver_ix],
            Some(&payer.pubkey()),
            &[&payer],
            last_blockhash,
        );
        let err = banks_client.process_transaction(tx).await.unwrap_err();
        assert_eq!(
            err.unwrap(),
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(ErrorCode::ConstraintRaw as u32)
            )
        );
    }

    #[tokio::test]
    async fn test_change_tip_receiver_different_block_builder_fails() {
        let ProgramTestContext {
            mut banks_client,
            last_blockhash,
            payer,
            ..
        } = get_test(&[]).await;

        initialize_program(&mut banks_client, &payer, last_blockhash).await;

        let config_pda = get_config_pda();
        let tip_pdas = get_tip_pdas();

        let change_tip_receiver_ix = Instruction {
            program_id: jito_tip_payment::id(),
            data: jito_tip_payment::instruction::ChangeTipReceiver {}.data(),
            accounts: jito_tip_payment::accounts::ChangeTipReceiver {
                config: config_pda.0,
                old_tip_receiver: payer.pubkey(),
                new_tip_receiver: Pubkey::new_unique(),
                block_builder: Pubkey::new_unique(), // bad block builder
                tip_payment_account_0: tip_pdas[0].0,
                tip_payment_account_1: tip_pdas[1].0,
                tip_payment_account_2: tip_pdas[2].0,
                tip_payment_account_3: tip_pdas[3].0,
                tip_payment_account_4: tip_pdas[4].0,
                tip_payment_account_5: tip_pdas[5].0,
                tip_payment_account_6: tip_pdas[6].0,
                tip_payment_account_7: tip_pdas[7].0,
                signer: payer.pubkey(),
                // placeholder: block_builder's constraint fails before this is ever read.
                validator_vote_account: Pubkey::new_unique(),
            }
            .to_account_metas(None),
        };
        let tx = Transaction::new_signed_with_payer(
            &[change_tip_receiver_ix],
            Some(&payer.pubkey()),
            &[&payer],
            last_blockhash,
        );
        let err = banks_client.process_transaction(tx).await.unwrap_err();
        assert_eq!(
            err.unwrap(),
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(ErrorCode::ConstraintRaw as u32)
            )
        );
    }

    #[tokio::test]
    async fn test_change_block_builder_different_block_builder_fails() {
        let ProgramTestContext {
            mut banks_client,
            last_blockhash,
            payer,
            ..
        } = get_test(&[]).await;

        initialize_program(&mut banks_client, &payer, last_blockhash).await;

        let config_pda = get_config_pda();
        let tip_pdas = get_tip_pdas();

        let change_block_builder_ix = Instruction {
            program_id: jito_tip_payment::id(),
            data: jito_tip_payment::instruction::ChangeBlockBuilder {
                block_builder_commission: 0,
            }
            .data(),
            accounts: jito_tip_payment::accounts::ChangeBlockBuilder {
                config: config_pda.0,
                tip_receiver: payer.pubkey(),
                old_block_builder: Pubkey::new_unique(), // bad block builder
                new_block_builder: Pubkey::new_unique(),
                tip_payment_account_0: tip_pdas[0].0,
                tip_payment_account_1: tip_pdas[1].0,
                tip_payment_account_2: tip_pdas[2].0,
                tip_payment_account_3: tip_pdas[3].0,
                tip_payment_account_4: tip_pdas[4].0,
                tip_payment_account_5: tip_pdas[5].0,
                tip_payment_account_6: tip_pdas[6].0,
                tip_payment_account_7: tip_pdas[7].0,
                signer: payer.pubkey(),
            }
            .to_account_metas(None),
        };
        let tx = Transaction::new_signed_with_payer(
            &[change_block_builder_ix],
            Some(&payer.pubkey()),
            &[&payer],
            last_blockhash,
        );
        let err = banks_client.process_transaction(tx).await.unwrap_err();
        assert_eq!(
            err.unwrap(),
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(ErrorCode::ConstraintRaw as u32)
            )
        );
    }

    #[tokio::test]
    async fn test_change_block_builder_different_tip_receiver_fails() {
        let ProgramTestContext {
            mut banks_client,
            last_blockhash,
            payer,
            ..
        } = get_test(&[]).await;

        initialize_program(&mut banks_client, &payer, last_blockhash).await;

        let config_pda = get_config_pda();
        let tip_pdas = get_tip_pdas();

        let change_block_builder_ix = Instruction {
            program_id: jito_tip_payment::id(),
            data: jito_tip_payment::instruction::ChangeBlockBuilder {
                block_builder_commission: 0,
            }
            .data(),
            accounts: jito_tip_payment::accounts::ChangeBlockBuilder {
                config: config_pda.0,
                tip_receiver: Pubkey::new_unique(), // bad tip receiver
                old_block_builder: payer.pubkey(),
                new_block_builder: Pubkey::new_unique(),
                tip_payment_account_0: tip_pdas[0].0,
                tip_payment_account_1: tip_pdas[1].0,
                tip_payment_account_2: tip_pdas[2].0,
                tip_payment_account_3: tip_pdas[3].0,
                tip_payment_account_4: tip_pdas[4].0,
                tip_payment_account_5: tip_pdas[5].0,
                tip_payment_account_6: tip_pdas[6].0,
                tip_payment_account_7: tip_pdas[7].0,
                signer: payer.pubkey(),
            }
            .to_account_metas(None),
        };
        let tx = Transaction::new_signed_with_payer(
            &[change_block_builder_ix],
            Some(&payer.pubkey()),
            &[&payer],
            last_blockhash,
        );
        let err = banks_client.process_transaction(tx).await.unwrap_err();
        assert_eq!(
            err.unwrap(),
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(ErrorCode::ConstraintRaw as u32)
            )
        );
    }

    #[tokio::test]
    async fn test_change_block_builder_bad_commission() {
        let ProgramTestContext {
            mut banks_client,
            last_blockhash,
            payer,
            ..
        } = get_test(&[]).await;

        initialize_program(&mut banks_client, &payer, last_blockhash).await;

        let config_pda = get_config_pda();
        let tip_pdas = get_tip_pdas();

        let change_block_builder_ix = Instruction {
            program_id: jito_tip_payment::id(),
            data: jito_tip_payment::instruction::ChangeBlockBuilder {
                block_builder_commission: 101,
            }
            .data(),
            accounts: jito_tip_payment::accounts::ChangeBlockBuilder {
                config: config_pda.0,
                tip_receiver: payer.pubkey(),
                old_block_builder: payer.pubkey(),
                new_block_builder: Pubkey::new_unique(),
                tip_payment_account_0: tip_pdas[0].0,
                tip_payment_account_1: tip_pdas[1].0,
                tip_payment_account_2: tip_pdas[2].0,
                tip_payment_account_3: tip_pdas[3].0,
                tip_payment_account_4: tip_pdas[4].0,
                tip_payment_account_5: tip_pdas[5].0,
                tip_payment_account_6: tip_pdas[6].0,
                tip_payment_account_7: tip_pdas[7].0,
                signer: payer.pubkey(),
            }
            .to_account_metas(None),
        };
        let tx = Transaction::new_signed_with_payer(
            &[change_block_builder_ix],
            Some(&payer.pubkey()),
            &[&payer],
            last_blockhash,
        );
        let err = banks_client.process_transaction(tx).await.unwrap_err();
        assert_eq!(
            err.unwrap(),
            TransactionError::InstructionError(0, InstructionError::Custom(6001))
        );
    }

    #[tokio::test]
    async fn test_change_tip_receiver_reserved_accounts() {
        let ProgramTestContext {
            mut banks_client,
            last_blockhash,
            payer,
            ..
        } = get_test(&[]).await;
        initialize_program(&mut banks_client, &payer, last_blockhash).await;

        let tip_pdas = get_tip_pdas();
        let config_pda = get_config_pda();

        for reserved_account in ReservedAccountKeys::all_keys_iter() {
            let change_tip_receiver_ix = Instruction {
                program_id: jito_tip_payment::id(),
                data: jito_tip_payment::instruction::ChangeTipReceiver {}.data(),
                accounts: jito_tip_payment::accounts::ChangeTipReceiver {
                    config: config_pda.0,
                    old_tip_receiver: payer.pubkey(),
                    new_tip_receiver: *reserved_account, // reserved account
                    block_builder: payer.pubkey(),
                    tip_payment_account_0: tip_pdas[0].0,
                    tip_payment_account_1: tip_pdas[1].0,
                    tip_payment_account_2: tip_pdas[2].0,
                    tip_payment_account_3: tip_pdas[3].0,
                    tip_payment_account_4: tip_pdas[4].0,
                    tip_payment_account_5: tip_pdas[5].0,
                    tip_payment_account_6: tip_pdas[6].0,
                    tip_payment_account_7: tip_pdas[7].0,
                    signer: payer.pubkey(),
                    // placeholder: new_tip_receiver's mut constraint fails (demoted reserved
                    // account) before this is ever read.
                    validator_vote_account: Pubkey::new_unique(),
                }
                .to_account_metas(None),
            };
            let tx = Transaction::new_signed_with_payer(
                &[change_tip_receiver_ix],
                Some(&payer.pubkey()),
                &[&payer],
                last_blockhash,
            );
            let err = banks_client
                .process_transaction(tx)
                .await
                .unwrap_err()
                .unwrap();
            assert_eq!(
                err,
                TransactionError::InstructionError(
                    0,
                    InstructionError::Custom(ErrorCode::ConstraintMut as u32) // reserved accounts are demoted to read lock
                )
            );
        }
    }

    #[tokio::test]
    async fn test_change_block_builder_reserved_accounts() {
        let ProgramTestContext {
            mut banks_client,
            last_blockhash,
            payer,
            ..
        } = get_test(&[]).await;
        initialize_program(&mut banks_client, &payer, last_blockhash).await;

        let tip_pdas = get_tip_pdas();
        let config_pda = get_config_pda();

        for reserved_account in ReservedAccountKeys::all_keys_iter() {
            let change_block_builder_ix = Instruction {
                program_id: jito_tip_payment::id(),
                data: jito_tip_payment::instruction::ChangeBlockBuilder {
                    block_builder_commission: 0,
                }
                .data(),
                accounts: jito_tip_payment::accounts::ChangeBlockBuilder {
                    config: config_pda.0,
                    tip_receiver: payer.pubkey(),
                    old_block_builder: payer.pubkey(),
                    new_block_builder: *reserved_account,
                    tip_payment_account_0: tip_pdas[0].0,
                    tip_payment_account_1: tip_pdas[1].0,
                    tip_payment_account_2: tip_pdas[2].0,
                    tip_payment_account_3: tip_pdas[3].0,
                    tip_payment_account_4: tip_pdas[4].0,
                    tip_payment_account_5: tip_pdas[5].0,
                    tip_payment_account_6: tip_pdas[6].0,
                    tip_payment_account_7: tip_pdas[7].0,
                    signer: payer.pubkey(),
                }
                .to_account_metas(None),
            };
            let tx = Transaction::new_signed_with_payer(
                &[change_block_builder_ix],
                Some(&payer.pubkey()),
                &[&payer],
                last_blockhash,
            );
            let err = banks_client
                .process_transaction(tx)
                .await
                .unwrap_err()
                .unwrap();
            assert_eq!(
                err,
                TransactionError::InstructionError(
                    0,
                    InstructionError::Custom(ErrorCode::ConstraintMut as u32) // reserved accounts are demoted to read locks
                )
            );
        }
    }

    #[tokio::test]
    async fn test_change_tip_receiver_to_tip_payment_program_demote_lock_fails() {
        let ProgramTestContext {
            mut banks_client,
            last_blockhash,
            payer,
            ..
        } = get_test(&[]).await;
        initialize_program(&mut banks_client, &payer, last_blockhash).await;

        let tip_pdas = get_tip_pdas();
        let config_pda = get_config_pda();

        let change_tip_receiver_ix = Instruction {
            program_id: jito_tip_payment::id(),
            data: jito_tip_payment::instruction::ChangeTipReceiver {}.data(),
            accounts: jito_tip_payment::accounts::ChangeTipReceiver {
                config: config_pda.0,
                old_tip_receiver: payer.pubkey(),
                new_tip_receiver: jito_tip_payment::id(), // demoted to read lock
                block_builder: payer.pubkey(),
                tip_payment_account_0: tip_pdas[0].0,
                tip_payment_account_1: tip_pdas[1].0,
                tip_payment_account_2: tip_pdas[2].0,
                tip_payment_account_3: tip_pdas[3].0,
                tip_payment_account_4: tip_pdas[4].0,
                tip_payment_account_5: tip_pdas[5].0,
                tip_payment_account_6: tip_pdas[6].0,
                tip_payment_account_7: tip_pdas[7].0,
                signer: payer.pubkey(),
                // placeholder: new_tip_receiver's mut constraint fails (demoted executable
                // account) before this is ever read.
                validator_vote_account: Pubkey::new_unique(),
            }
            .to_account_metas(None),
        };
        let tx = Transaction::new_signed_with_payer(
            &[change_tip_receiver_ix],
            Some(&payer.pubkey()),
            &[&payer],
            last_blockhash,
        );
        let err = banks_client.process_transaction(tx).await.unwrap_err();
        assert_eq!(
            err.unwrap(),
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(ErrorCode::ConstraintMut as u32)
            )
        );
    }

    #[tokio::test]
    async fn test_change_tip_receiver_to_tip_payment_program_with_loader_fails() {
        let validator_identity = Keypair::new();
        let vote_account = Pubkey::new_unique();

        let ProgramTestContext {
            mut banks_client,
            last_blockhash,
            payer,
            ..
        } = get_test(&[(
            vote_account,
            build_fake_vote_account(&validator_identity.pubkey()),
        )])
        .await;
        initialize_program(&mut banks_client, &payer, last_blockhash).await;

        let tip_pdas = get_tip_pdas();
        let config_pda = get_config_pda();

        let mut change_tip_receiver_ix = Instruction {
            program_id: jito_tip_payment::id(),
            data: jito_tip_payment::instruction::ChangeTipReceiver {}.data(),
            accounts: jito_tip_payment::accounts::ChangeTipReceiver {
                config: config_pda.0,
                old_tip_receiver: payer.pubkey(),
                new_tip_receiver: jito_tip_payment::id(),
                block_builder: payer.pubkey(),
                tip_payment_account_0: tip_pdas[0].0,
                tip_payment_account_1: tip_pdas[1].0,
                tip_payment_account_2: tip_pdas[2].0,
                tip_payment_account_3: tip_pdas[3].0,
                tip_payment_account_4: tip_pdas[4].0,
                tip_payment_account_5: tip_pdas[5].0,
                tip_payment_account_6: tip_pdas[6].0,
                tip_payment_account_7: tip_pdas[7].0,
                signer: validator_identity.pubkey(),
                validator_vote_account: vote_account,
            }
            .to_account_metas(None),
        };
        change_tip_receiver_ix
            .accounts
            .push(AccountMeta::new(bpf_loader_upgradeable::id(), false));
        let tx = Transaction::new_signed_with_payer(
            &[change_tip_receiver_ix],
            Some(&payer.pubkey()),
            &[&payer, &validator_identity],
            last_blockhash,
        );
        let err = banks_client.process_transaction(tx).await.unwrap_err();
        // `new_tip_receiver` is the program's own id here — previously rejected by the
        // handler's `is_program` check (InvalidTipReceiver). The new PDA-binding check now
        // runs first and rejects it earlier: a program id can never equal a validly-derived
        // TDA PDA, so this gap is closed as a side effect (see plan doc).
        assert_eq!(
            err.unwrap(),
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    ERROR_CODE_OFFSET + TipPaymentError::TipReceiverPdaMismatch as u32
                )
            )
        );
    }

    #[tokio::test]
    async fn test_change_block_builder_to_tip_payment_program_demote_lock_fails() {
        let ProgramTestContext {
            mut banks_client,
            last_blockhash,
            payer,
            ..
        } = get_test(&[]).await;
        initialize_program(&mut banks_client, &payer, last_blockhash).await;

        let tip_pdas = get_tip_pdas();
        let config_pda = get_config_pda();

        let change_block_builder_ix = Instruction {
            program_id: jito_tip_payment::id(),
            data: jito_tip_payment::instruction::ChangeBlockBuilder {
                block_builder_commission: 0,
            }
            .data(),
            accounts: jito_tip_payment::accounts::ChangeBlockBuilder {
                config: config_pda.0,
                tip_receiver: payer.pubkey(),
                old_block_builder: payer.pubkey(),
                new_block_builder: jito_tip_payment::id(),
                tip_payment_account_0: tip_pdas[0].0,
                tip_payment_account_1: tip_pdas[1].0,
                tip_payment_account_2: tip_pdas[2].0,
                tip_payment_account_3: tip_pdas[3].0,
                tip_payment_account_4: tip_pdas[4].0,
                tip_payment_account_5: tip_pdas[5].0,
                tip_payment_account_6: tip_pdas[6].0,
                tip_payment_account_7: tip_pdas[7].0,
                signer: payer.pubkey(),
            }
            .to_account_metas(None),
        };
        let tx = Transaction::new_signed_with_payer(
            &[change_block_builder_ix],
            Some(&payer.pubkey()),
            &[&payer],
            last_blockhash,
        );
        let err = banks_client.process_transaction(tx).await.unwrap_err();
        assert_eq!(
            err.unwrap(),
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(ErrorCode::ConstraintMut as u32)
            )
        );
    }

    #[tokio::test]
    async fn test_change_block_builder_to_tip_payment_program_with_loader_fails() {
        let ProgramTestContext {
            mut banks_client,
            last_blockhash,
            payer,
            ..
        } = get_test(&[]).await;
        initialize_program(&mut banks_client, &payer, last_blockhash).await;

        let tip_pdas = get_tip_pdas();
        let config_pda = get_config_pda();

        let mut change_block_builder_ix = Instruction {
            program_id: jito_tip_payment::id(),
            data: jito_tip_payment::instruction::ChangeBlockBuilder {
                block_builder_commission: 0,
            }
            .data(),
            accounts: jito_tip_payment::accounts::ChangeBlockBuilder {
                config: config_pda.0,
                tip_receiver: payer.pubkey(),
                old_block_builder: payer.pubkey(),
                new_block_builder: jito_tip_payment::id(),
                tip_payment_account_0: tip_pdas[0].0,
                tip_payment_account_1: tip_pdas[1].0,
                tip_payment_account_2: tip_pdas[2].0,
                tip_payment_account_3: tip_pdas[3].0,
                tip_payment_account_4: tip_pdas[4].0,
                tip_payment_account_5: tip_pdas[5].0,
                tip_payment_account_6: tip_pdas[6].0,
                tip_payment_account_7: tip_pdas[7].0,
                signer: payer.pubkey(),
            }
            .to_account_metas(None),
        };
        change_block_builder_ix
            .accounts
            .push(AccountMeta::new(bpf_loader_upgradeable::id(), false));
        let tx = Transaction::new_signed_with_payer(
            &[change_block_builder_ix],
            Some(&payer.pubkey()),
            &[&payer],
            last_blockhash,
        );
        let err = banks_client.process_transaction(tx).await.unwrap_err();
        assert_eq!(
            err.unwrap(),
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    ERROR_CODE_OFFSET + TipPaymentError::InvalidBlockBuilder as u32
                )
            )
        );
    }

    #[tokio::test]
    async fn test_change_block_builder_below_rent_exempt_block_builder_ok() {
        let ProgramTestContext {
            mut banks_client,
            last_blockhash,
            payer,
            ..
        } = get_test(&[]).await;
        initialize_program(&mut banks_client, &payer, last_blockhash).await;

        let config_pda = get_config_pda();
        let tip_pdas = get_tip_pdas();

        // sets the block builder commission to 100%, which takes effect AFTER the instruction is
        // processed
        let new_block_builder_1 = Pubkey::new_unique();
        let set_block_builder_commission = Instruction {
            program_id: jito_tip_payment::id(),
            data: jito_tip_payment::instruction::ChangeBlockBuilder {
                block_builder_commission: 100,
            }
            .data(),
            accounts: jito_tip_payment::accounts::ChangeBlockBuilder {
                config: config_pda.0,
                tip_receiver: payer.pubkey(),
                old_block_builder: payer.pubkey(),
                new_block_builder: new_block_builder_1,
                tip_payment_account_0: tip_pdas[0].0,
                tip_payment_account_1: tip_pdas[1].0,
                tip_payment_account_2: tip_pdas[2].0,
                tip_payment_account_3: tip_pdas[3].0,
                tip_payment_account_4: tip_pdas[4].0,
                tip_payment_account_5: tip_pdas[5].0,
                tip_payment_account_6: tip_pdas[6].0,
                tip_payment_account_7: tip_pdas[7].0,
                signer: payer.pubkey(),
            }
            .to_account_metas(None),
        };
        let tx = Transaction::new_signed_with_payer(
            &[
                set_block_builder_commission,
                transfer(&payer.pubkey(), &tip_pdas[1].0, 1),
            ],
            Some(&payer.pubkey()),
            &[&payer],
            last_blockhash,
        );
        banks_client
            .process_transaction_with_commitment(tx, CommitmentLevel::Processed)
            .await
            .unwrap();

        let mut tip_pda_balances_before = vec![];
        for (pubkey, _) in &tip_pdas {
            tip_pda_balances_before.push(
                banks_client
                    .get_account_with_commitment(*pubkey, CommitmentLevel::Processed)
                    .await
                    .unwrap()
                    .unwrap()
                    .lamports,
            );
        }

        let new_block_builder_2 = Pubkey::new_unique();
        let change_block_builder_ix = Instruction {
            program_id: jito_tip_payment::id(),
            data: jito_tip_payment::instruction::ChangeBlockBuilder {
                block_builder_commission: 100,
            }
            .data(),
            accounts: jito_tip_payment::accounts::ChangeBlockBuilder {
                config: config_pda.0,
                tip_receiver: payer.pubkey(),
                old_block_builder: new_block_builder_1,
                new_block_builder: new_block_builder_2,
                tip_payment_account_0: tip_pdas[0].0,
                tip_payment_account_1: tip_pdas[1].0,
                tip_payment_account_2: tip_pdas[2].0,
                tip_payment_account_3: tip_pdas[3].0,
                tip_payment_account_4: tip_pdas[4].0,
                tip_payment_account_5: tip_pdas[5].0,
                tip_payment_account_6: tip_pdas[6].0,
                tip_payment_account_7: tip_pdas[7].0,
                signer: payer.pubkey(),
            }
            .to_account_metas(None),
        };

        // send some lamports to a tip account then try to change the block builder account
        let tx = Transaction::new_signed_with_payer(
            &[change_block_builder_ix],
            Some(&payer.pubkey()),
            &[&payer],
            last_blockhash,
        );
        banks_client
            .process_transaction_with_commitment(tx, CommitmentLevel::Processed)
            .await
            .unwrap();

        let config_account_after = banks_client
            .get_account_with_commitment(config_pda.0, CommitmentLevel::Processed)
            .await
            .unwrap()
            .unwrap();
        let config_account_after =
            Config::try_deserialize(&mut config_account_after.data()).unwrap();
        assert_eq!(config_account_after.block_builder, new_block_builder_2);

        // assert nothing transferred
        assert!(banks_client
            .get_account_with_commitment(new_block_builder_1, CommitmentLevel::Processed)
            .await
            .unwrap()
            .is_none());
        assert!(banks_client
            .get_account_with_commitment(new_block_builder_2, CommitmentLevel::Processed)
            .await
            .unwrap()
            .is_none());

        let mut tip_pda_balances_after = vec![];
        for (pubkey, _) in &tip_pdas {
            tip_pda_balances_after.push(
                banks_client
                    .get_account_with_commitment(*pubkey, CommitmentLevel::Processed)
                    .await
                    .unwrap()
                    .unwrap()
                    .lamports,
            );
        }

        assert_eq!(
            tip_pda_balances_after.iter().sum::<u64>(),
            tip_pda_balances_before.iter().sum::<u64>()
        );

        // make sure the lamports moved to tip pda 0
        assert_eq!(tip_pda_balances_after[0] - tip_pda_balances_before[0], 1);
        assert_eq!(tip_pda_balances_before[1] - tip_pda_balances_after[1], 1);
    }

    #[tokio::test]
    async fn test_change_block_builder_below_rent_exempt_tip_receiver() {
        let validator_identity = Keypair::new();
        let vote_account = Pubkey::new_unique();

        let ProgramTestContext {
            mut banks_client,
            last_blockhash,
            payer,
            ..
        } = get_test(&[(
            vote_account,
            build_fake_vote_account(&validator_identity.pubkey()),
        )])
        .await;
        initialize_program(&mut banks_client, &payer, last_blockhash).await;

        let config_pda = get_config_pda();
        let tip_pdas = get_tip_pdas();

        // set the tip receiver and transfer 1 lamport to the tip pda 1
        let epoch = get_epoch(&mut banks_client).await;
        let new_tip_receiver = get_tip_distribution_pda(&vote_account, epoch);
        banks_client
            .process_transaction_with_commitment(
                Transaction::new_signed_with_payer(
                    &[
                        Instruction {
                            program_id: jito_tip_payment::id(),
                            data: jito_tip_payment::instruction::ChangeTipReceiver {}.data(),
                            accounts: jito_tip_payment::accounts::ChangeTipReceiver {
                                config: config_pda.0,
                                old_tip_receiver: payer.pubkey(),
                                new_tip_receiver,
                                block_builder: payer.pubkey(),
                                tip_payment_account_0: tip_pdas[0].0,
                                tip_payment_account_1: tip_pdas[1].0,
                                tip_payment_account_2: tip_pdas[2].0,
                                tip_payment_account_3: tip_pdas[3].0,
                                tip_payment_account_4: tip_pdas[4].0,
                                tip_payment_account_5: tip_pdas[5].0,
                                tip_payment_account_6: tip_pdas[6].0,
                                tip_payment_account_7: tip_pdas[7].0,
                                signer: validator_identity.pubkey(),
                                validator_vote_account: vote_account,
                            }
                            .to_account_metas(None),
                        },
                        transfer(&payer.pubkey(), &tip_pdas[1].0, 1),
                    ],
                    Some(&payer.pubkey()),
                    &[&payer, &validator_identity],
                    last_blockhash,
                ),
                CommitmentLevel::Processed,
            )
            .await
            .unwrap();

        let mut tip_pda_balances_before = vec![];
        for (pubkey, _) in &tip_pdas {
            tip_pda_balances_before.push(
                banks_client
                    .get_account_with_commitment(*pubkey, CommitmentLevel::Processed)
                    .await
                    .unwrap()
                    .unwrap()
                    .lamports,
            );
        }

        banks_client
            .process_transaction_with_commitment(
                Transaction::new_signed_with_payer(
                    &[Instruction {
                        program_id: jito_tip_payment::id(),
                        data: jito_tip_payment::instruction::ChangeBlockBuilder {
                            block_builder_commission: 0,
                        }
                        .data(),
                        accounts: jito_tip_payment::accounts::ChangeBlockBuilder {
                            config: config_pda.0,
                            tip_receiver: new_tip_receiver,
                            old_block_builder: payer.pubkey(),
                            new_block_builder: Pubkey::new_unique(),
                            tip_payment_account_0: tip_pdas[0].0,
                            tip_payment_account_1: tip_pdas[1].0,
                            tip_payment_account_2: tip_pdas[2].0,
                            tip_payment_account_3: tip_pdas[3].0,
                            tip_payment_account_4: tip_pdas[4].0,
                            tip_payment_account_5: tip_pdas[5].0,
                            tip_payment_account_6: tip_pdas[6].0,
                            tip_payment_account_7: tip_pdas[7].0,
                            signer: payer.pubkey(),
                        }
                        .to_account_metas(None),
                    }],
                    Some(&payer.pubkey()),
                    &[&payer],
                    last_blockhash,
                ),
                CommitmentLevel::Processed,
            )
            .await
            .unwrap();

        // assert nothing transferred to the new tip receiver
        assert!(banks_client
            .get_account_with_commitment(new_tip_receiver, CommitmentLevel::Processed)
            .await
            .unwrap()
            .is_none());

        let mut tip_pda_balances_after = vec![];
        for (pubkey, _) in &tip_pdas {
            tip_pda_balances_after.push(
                banks_client
                    .get_account_with_commitment(*pubkey, CommitmentLevel::Processed)
                    .await
                    .unwrap()
                    .unwrap()
                    .lamports,
            );
        }

        assert_eq!(
            tip_pda_balances_after.iter().sum::<u64>(),
            tip_pda_balances_before.iter().sum::<u64>()
        );

        // make sure the lamports moved to tip pda 0
        assert_eq!(tip_pda_balances_after[0] - tip_pda_balances_before[0], 1);
        assert_eq!(tip_pda_balances_before[1] - tip_pda_balances_after[1], 1);
    }

    #[tokio::test]
    async fn test_change_tip_receiver_below_rent_exempt_tip_receiver() {
        let validator_identity = Keypair::new();
        let vote_account_a = Pubkey::new_unique();
        let vote_account_b = Pubkey::new_unique();

        let ProgramTestContext {
            mut banks_client,
            last_blockhash,
            payer,
            ..
        } = get_test(&[
            (
                vote_account_a,
                build_fake_vote_account(&validator_identity.pubkey()),
            ),
            (
                vote_account_b,
                build_fake_vote_account(&validator_identity.pubkey()),
            ),
        ])
        .await;
        initialize_program(&mut banks_client, &payer, last_blockhash).await;

        let config_pda = get_config_pda();
        let tip_pdas = get_tip_pdas();

        // set the tip receiver and transfer 1 lamport to the tip pda 1
        let epoch = get_epoch(&mut banks_client).await;
        let new_tip_receiver = get_tip_distribution_pda(&vote_account_a, epoch);
        banks_client
            .process_transaction_with_commitment(
                Transaction::new_signed_with_payer(
                    &[
                        Instruction {
                            program_id: jito_tip_payment::id(),
                            data: jito_tip_payment::instruction::ChangeTipReceiver {}.data(),
                            accounts: jito_tip_payment::accounts::ChangeTipReceiver {
                                config: config_pda.0,
                                old_tip_receiver: payer.pubkey(),
                                new_tip_receiver,
                                block_builder: payer.pubkey(),
                                tip_payment_account_0: tip_pdas[0].0,
                                tip_payment_account_1: tip_pdas[1].0,
                                tip_payment_account_2: tip_pdas[2].0,
                                tip_payment_account_3: tip_pdas[3].0,
                                tip_payment_account_4: tip_pdas[4].0,
                                tip_payment_account_5: tip_pdas[5].0,
                                tip_payment_account_6: tip_pdas[6].0,
                                tip_payment_account_7: tip_pdas[7].0,
                                signer: validator_identity.pubkey(),
                                validator_vote_account: vote_account_a,
                            }
                            .to_account_metas(None),
                        },
                        transfer(&payer.pubkey(), &tip_pdas[1].0, 1),
                    ],
                    Some(&payer.pubkey()),
                    &[&payer, &validator_identity],
                    last_blockhash,
                ),
                CommitmentLevel::Processed,
            )
            .await
            .unwrap();

        let mut tip_pda_balances_before = vec![];
        for (pubkey, _) in &tip_pdas {
            tip_pda_balances_before.push(
                banks_client
                    .get_account_with_commitment(*pubkey, CommitmentLevel::Processed)
                    .await
                    .unwrap()
                    .unwrap()
                    .lamports,
            );
        }

        let newer_tip_receiver = get_tip_distribution_pda(&vote_account_b, epoch);
        banks_client
            .process_transaction_with_commitment(
                Transaction::new_signed_with_payer(
                    &[Instruction {
                        program_id: jito_tip_payment::id(),
                        data: jito_tip_payment::instruction::ChangeTipReceiver {}.data(),
                        accounts: jito_tip_payment::accounts::ChangeTipReceiver {
                            config: config_pda.0,
                            old_tip_receiver: new_tip_receiver,
                            new_tip_receiver: newer_tip_receiver,
                            block_builder: payer.pubkey(),
                            tip_payment_account_0: tip_pdas[0].0,
                            tip_payment_account_1: tip_pdas[1].0,
                            tip_payment_account_2: tip_pdas[2].0,
                            tip_payment_account_3: tip_pdas[3].0,
                            tip_payment_account_4: tip_pdas[4].0,
                            tip_payment_account_5: tip_pdas[5].0,
                            tip_payment_account_6: tip_pdas[6].0,
                            tip_payment_account_7: tip_pdas[7].0,
                            signer: validator_identity.pubkey(),
                            validator_vote_account: vote_account_b,
                        }
                        .to_account_metas(None),
                    }],
                    Some(&payer.pubkey()),
                    &[&payer, &validator_identity],
                    last_blockhash,
                ),
                CommitmentLevel::Processed,
            )
            .await
            .unwrap();

        // assert nothing transferred to the new tip receiver
        assert!(banks_client
            .get_account_with_commitment(new_tip_receiver, CommitmentLevel::Processed)
            .await
            .unwrap()
            .is_none());

        let mut tip_pda_balances_after = vec![];
        for (pubkey, _) in &tip_pdas {
            tip_pda_balances_after.push(
                banks_client
                    .get_account_with_commitment(*pubkey, CommitmentLevel::Processed)
                    .await
                    .unwrap()
                    .unwrap()
                    .lamports,
            );
        }

        assert_eq!(
            tip_pda_balances_after.iter().sum::<u64>(),
            tip_pda_balances_before.iter().sum::<u64>()
        );

        // make sure the lamports moved to tip pda 0
        assert_eq!(tip_pda_balances_after[0] - tip_pda_balances_before[0], 1);
        assert_eq!(tip_pda_balances_before[1] - tip_pda_balances_after[1], 1);
    }

    #[tokio::test]
    async fn test_change_tip_receiver_below_rent_exempt_block_builder() {
        let validator_identity = Keypair::new();
        let vote_account = Pubkey::new_unique();

        let ProgramTestContext {
            mut banks_client,
            last_blockhash,
            payer,
            ..
        } = get_test(&[(
            vote_account,
            build_fake_vote_account(&validator_identity.pubkey()),
        )])
        .await;
        initialize_program(&mut banks_client, &payer, last_blockhash).await;

        let config_pda = get_config_pda();
        let tip_pdas = get_tip_pdas();

        // set the tip receiver and transfer 1 lamport to the tip pda 1
        let new_block_builder = Pubkey::new_unique();
        banks_client
            .process_transaction_with_commitment(
                Transaction::new_signed_with_payer(
                    &[
                        Instruction {
                            program_id: jito_tip_payment::id(),
                            data: jito_tip_payment::instruction::ChangeBlockBuilder {
                                block_builder_commission: 100,
                            }
                            .data(),
                            accounts: jito_tip_payment::accounts::ChangeBlockBuilder {
                                config: config_pda.0,
                                tip_receiver: payer.pubkey(),
                                old_block_builder: payer.pubkey(),
                                new_block_builder,
                                tip_payment_account_0: tip_pdas[0].0,
                                tip_payment_account_1: tip_pdas[1].0,
                                tip_payment_account_2: tip_pdas[2].0,
                                tip_payment_account_3: tip_pdas[3].0,
                                tip_payment_account_4: tip_pdas[4].0,
                                tip_payment_account_5: tip_pdas[5].0,
                                tip_payment_account_6: tip_pdas[6].0,
                                tip_payment_account_7: tip_pdas[7].0,
                                signer: payer.pubkey(),
                            }
                            .to_account_metas(None),
                        },
                        transfer(&payer.pubkey(), &tip_pdas[1].0, 1),
                    ],
                    Some(&payer.pubkey()),
                    &[&payer],
                    last_blockhash,
                ),
                CommitmentLevel::Processed,
            )
            .await
            .unwrap();

        let mut tip_pda_balances_before = vec![];
        for (pubkey, _) in &tip_pdas {
            tip_pda_balances_before.push(
                banks_client
                    .get_account_with_commitment(*pubkey, CommitmentLevel::Processed)
                    .await
                    .unwrap()
                    .unwrap()
                    .lamports,
            );
        }

        let epoch = get_epoch(&mut banks_client).await;
        let new_tip_receiver = get_tip_distribution_pda(&vote_account, epoch);
        banks_client
            .process_transaction_with_commitment(
                Transaction::new_signed_with_payer(
                    &[Instruction {
                        program_id: jito_tip_payment::id(),
                        data: jito_tip_payment::instruction::ChangeTipReceiver {}.data(),
                        accounts: jito_tip_payment::accounts::ChangeTipReceiver {
                            config: config_pda.0,
                            old_tip_receiver: payer.pubkey(),
                            new_tip_receiver,
                            block_builder: new_block_builder,
                            tip_payment_account_0: tip_pdas[0].0,
                            tip_payment_account_1: tip_pdas[1].0,
                            tip_payment_account_2: tip_pdas[2].0,
                            tip_payment_account_3: tip_pdas[3].0,
                            tip_payment_account_4: tip_pdas[4].0,
                            tip_payment_account_5: tip_pdas[5].0,
                            tip_payment_account_6: tip_pdas[6].0,
                            tip_payment_account_7: tip_pdas[7].0,
                            signer: validator_identity.pubkey(),
                            validator_vote_account: vote_account,
                        }
                        .to_account_metas(None),
                    }],
                    Some(&payer.pubkey()),
                    &[&payer, &validator_identity],
                    last_blockhash,
                ),
                CommitmentLevel::Processed,
            )
            .await
            .unwrap();

        // assert nothing transferred to the new tip receiver
        assert!(banks_client
            .get_account_with_commitment(new_block_builder, CommitmentLevel::Processed)
            .await
            .unwrap()
            .is_none());

        let mut tip_pda_balances_after = vec![];
        for (pubkey, _) in &tip_pdas {
            tip_pda_balances_after.push(
                banks_client
                    .get_account_with_commitment(*pubkey, CommitmentLevel::Processed)
                    .await
                    .unwrap()
                    .unwrap()
                    .lamports,
            );
        }

        assert_eq!(
            tip_pda_balances_after.iter().sum::<u64>(),
            tip_pda_balances_before.iter().sum::<u64>()
        );

        // make sure the lamports moved to tip pda 0
        assert_eq!(tip_pda_balances_after[0] - tip_pda_balances_before[0], 1);
        assert_eq!(tip_pda_balances_before[1] - tip_pda_balances_after[1], 1);
    }

    #[tokio::test]
    async fn test_fees_bias_tip_receiver() {
        let validator_identity = Keypair::new();
        let vote_account = Pubkey::new_unique();

        let ProgramTestContext {
            mut banks_client,
            last_blockhash,
            payer,
            ..
        } = get_test(&[(
            vote_account,
            build_fake_vote_account(&validator_identity.pubkey()),
        )])
        .await;
        initialize_program(&mut banks_client, &payer, last_blockhash).await;

        let config_pda = get_config_pda();
        let tip_pdas = get_tip_pdas();

        // set the tip receiver and transfer 1 lamport to the tip pda 1
        let new_block_builder = Pubkey::new_unique();
        let epoch = get_epoch(&mut banks_client).await;
        let new_tip_receiver = get_tip_distribution_pda(&vote_account, epoch);
        banks_client
            .process_transaction_with_commitment(
                Transaction::new_signed_with_payer(
                    &[
                        Instruction {
                            program_id: jito_tip_payment::id(),
                            data: jito_tip_payment::instruction::ChangeBlockBuilder {
                                block_builder_commission: 50,
                            }
                            .data(),
                            accounts: jito_tip_payment::accounts::ChangeBlockBuilder {
                                config: config_pda.0,
                                tip_receiver: payer.pubkey(),
                                old_block_builder: payer.pubkey(),
                                new_block_builder,
                                tip_payment_account_0: tip_pdas[0].0,
                                tip_payment_account_1: tip_pdas[1].0,
                                tip_payment_account_2: tip_pdas[2].0,
                                tip_payment_account_3: tip_pdas[3].0,
                                tip_payment_account_4: tip_pdas[4].0,
                                tip_payment_account_5: tip_pdas[5].0,
                                tip_payment_account_6: tip_pdas[6].0,
                                tip_payment_account_7: tip_pdas[7].0,
                                signer: payer.pubkey(),
                            }
                            .to_account_metas(None),
                        },
                        Instruction {
                            program_id: jito_tip_payment::id(),
                            data: jito_tip_payment::instruction::ChangeTipReceiver {}.data(),
                            accounts: jito_tip_payment::accounts::ChangeTipReceiver {
                                config: config_pda.0,
                                old_tip_receiver: payer.pubkey(),
                                new_tip_receiver,
                                block_builder: new_block_builder,
                                tip_payment_account_0: tip_pdas[0].0,
                                tip_payment_account_1: tip_pdas[1].0,
                                tip_payment_account_2: tip_pdas[2].0,
                                tip_payment_account_3: tip_pdas[3].0,
                                tip_payment_account_4: tip_pdas[4].0,
                                tip_payment_account_5: tip_pdas[5].0,
                                tip_payment_account_6: tip_pdas[6].0,
                                tip_payment_account_7: tip_pdas[7].0,
                                signer: validator_identity.pubkey(),
                                validator_vote_account: vote_account,
                            }
                            .to_account_metas(None),
                        },
                        transfer(&payer.pubkey(), &tip_pdas[1].0, 333_333_333),
                    ],
                    Some(&payer.pubkey()),
                    &[&payer, &validator_identity],
                    last_blockhash,
                ),
                CommitmentLevel::Processed,
            )
            .await
            .unwrap();

        banks_client
            .process_transaction_with_commitment(
                Transaction::new_signed_with_payer(
                    &[Instruction {
                        program_id: jito_tip_payment::id(),
                        data: jito_tip_payment::instruction::ChangeTipReceiver {}.data(),
                        accounts: jito_tip_payment::accounts::ChangeTipReceiver {
                            config: config_pda.0,
                            old_tip_receiver: new_tip_receiver,
                            new_tip_receiver,
                            block_builder: new_block_builder,
                            tip_payment_account_0: tip_pdas[0].0,
                            tip_payment_account_1: tip_pdas[1].0,
                            tip_payment_account_2: tip_pdas[2].0,
                            tip_payment_account_3: tip_pdas[3].0,
                            tip_payment_account_4: tip_pdas[4].0,
                            tip_payment_account_5: tip_pdas[5].0,
                            tip_payment_account_6: tip_pdas[6].0,
                            tip_payment_account_7: tip_pdas[7].0,
                            signer: validator_identity.pubkey(),
                            validator_vote_account: vote_account,
                        }
                        .to_account_metas(None),
                    }],
                    Some(&payer.pubkey()),
                    &[&payer, &validator_identity],
                    last_blockhash,
                ),
                CommitmentLevel::Processed,
            )
            .await
            .unwrap();

        let block_builder_balance = banks_client
            .get_account_with_commitment(new_block_builder, CommitmentLevel::Processed)
            .await
            .unwrap()
            .unwrap()
            .lamports;
        let new_tip_receiver_balance = banks_client
            .get_account_with_commitment(new_tip_receiver, CommitmentLevel::Processed)
            .await
            .unwrap()
            .unwrap()
            .lamports;

        assert_eq!(block_builder_balance, 166_666_666);
        assert_eq!(new_tip_receiver_balance, 166_666_667);
    }

    fn get_initial_accounts(config: Config) -> Vec<(Pubkey, Account)> {
        let config_data = config.try_to_vec().unwrap();
        let mut serialized_config = Vec::with_capacity(8 + config_data.len());
        serialized_config.extend_from_slice(&Config::DISCRIMINATOR);
        serialized_config.extend_from_slice(&config_data);
        let (config_pubkey, _config_bump) =
            Pubkey::find_program_address(&[&CONFIG_ACCOUNT_SEED], &jito_tip_payment::id());

        let tpas = get_tip_pdas();
        let mut tpa_data = Vec::with_capacity(8);
        tpa_data.extend_from_slice(TipPaymentAccount::DISCRIMINATOR);
        let initial_accounts = vec![
            (
                config_pubkey,
                Account {
                    lamports: Rent::default().minimum_balance(serialized_config.len()),
                    data: serialized_config,
                    owner: jito_tip_payment::id(),
                    executable: false,
                    rent_epoch: u64::MAX,
                },
            ),
            (
                tpas[0].0,
                Account {
                    lamports: Rent::default().minimum_balance(8),
                    data: tpa_data.clone(),
                    owner: jito_tip_payment::id(),
                    executable: false,
                    rent_epoch: u64::MAX,
                },
            ),
            (
                tpas[1].0,
                Account {
                    lamports: Rent::default().minimum_balance(8),
                    data: tpa_data.clone(),
                    owner: jito_tip_payment::id(),
                    executable: false,
                    rent_epoch: u64::MAX,
                },
            ),
            (
                tpas[2].0,
                Account {
                    lamports: Rent::default().minimum_balance(8),
                    data: tpa_data.clone(),
                    owner: jito_tip_payment::id(),
                    executable: false,
                    rent_epoch: u64::MAX,
                },
            ),
            (
                tpas[3].0,
                Account {
                    lamports: Rent::default().minimum_balance(8),
                    data: tpa_data.clone(),
                    owner: jito_tip_payment::id(),
                    executable: false,
                    rent_epoch: u64::MAX,
                },
            ),
            (
                tpas[4].0,
                Account {
                    lamports: Rent::default().minimum_balance(8),
                    data: tpa_data.clone(),
                    owner: jito_tip_payment::id(),
                    executable: false,
                    rent_epoch: u64::MAX,
                },
            ),
            (
                tpas[5].0,
                Account {
                    lamports: Rent::default().minimum_balance(8),
                    data: tpa_data.clone(),
                    owner: jito_tip_payment::id(),
                    executable: false,
                    rent_epoch: u64::MAX,
                },
            ),
            (
                tpas[6].0,
                Account {
                    lamports: Rent::default().minimum_balance(8),
                    data: tpa_data.clone(),
                    owner: jito_tip_payment::id(),
                    executable: false,
                    rent_epoch: u64::MAX,
                },
            ),
            (
                tpas[7].0,
                Account {
                    lamports: Rent::default().minimum_balance(8),
                    data: tpa_data,
                    owner: jito_tip_payment::id(),
                    executable: false,
                    rent_epoch: u64::MAX,
                },
            ),
        ];

        initial_accounts
    }

    #[tokio::test]
    async fn test_change_tip_receiver_pay_tip_receiver_program() {
        let (config_pubkey, config_bump) =
            Pubkey::find_program_address(&[&CONFIG_ACCOUNT_SEED], &jito_tip_payment::id());
        let tip_pdas = get_tip_pdas();
        let validator_identity = Keypair::new();
        let vote_account = Pubkey::new_unique();

        let block_builder = Keypair::new();
        let config = Config {
            tip_receiver: jito_tip_distribution::id(),
            block_builder: block_builder.pubkey(),
            block_builder_commission_pct: 50,
            bumps: InitBumps {
                config: config_bump,
                tip_payment_account_0: tip_pdas[0].1,
                tip_payment_account_1: tip_pdas[1].1,
                tip_payment_account_2: tip_pdas[2].1,
                tip_payment_account_3: tip_pdas[3].1,
                tip_payment_account_4: tip_pdas[4].1,
                tip_payment_account_5: tip_pdas[5].1,
                tip_payment_account_6: tip_pdas[6].1,
                tip_payment_account_7: tip_pdas[7].1,
            },
        };
        let mut initial_accounts = get_initial_accounts(config);
        initial_accounts.push((
            vote_account,
            build_fake_vote_account(&validator_identity.pubkey()),
        ));

        let ProgramTestContext {
            banks_client,
            last_blockhash,
            payer,
            ..
        } = get_test(&initial_accounts).await;

        let program_before = banks_client
            .get_account_with_commitment(jito_tip_distribution::id(), CommitmentLevel::Processed)
            .await
            .unwrap()
            .unwrap();

        let epoch = get_epoch(&banks_client).await;
        let new_tip_receiver = get_tip_distribution_pda(&vote_account, epoch);
        banks_client
            .process_transaction_with_commitment(
                Transaction::new_signed_with_payer(
                    &[
                        transfer(&payer.pubkey(), &tip_pdas[1].0, 1_000_000),
                        Instruction {
                            program_id: jito_tip_payment::id(),
                            data: jito_tip_payment::instruction::ChangeTipReceiver {}.data(),
                            accounts: jito_tip_payment::accounts::ChangeTipReceiver {
                                config: config_pubkey,
                                old_tip_receiver: jito_tip_distribution::id(),
                                new_tip_receiver,
                                block_builder: block_builder.pubkey(),
                                tip_payment_account_0: tip_pdas[0].0,
                                tip_payment_account_1: tip_pdas[1].0,
                                tip_payment_account_2: tip_pdas[2].0,
                                tip_payment_account_3: tip_pdas[3].0,
                                tip_payment_account_4: tip_pdas[4].0,
                                tip_payment_account_5: tip_pdas[5].0,
                                tip_payment_account_6: tip_pdas[6].0,
                                tip_payment_account_7: tip_pdas[7].0,
                                signer: validator_identity.pubkey(),
                                validator_vote_account: vote_account,
                            }
                            .to_account_metas(None),
                        },
                    ],
                    Some(&payer.pubkey()),
                    &[&payer, &validator_identity],
                    last_blockhash,
                ),
                CommitmentLevel::Processed,
            )
            .await
            .unwrap();

        let program_after = banks_client
            .get_account_with_commitment(jito_tip_distribution::id(), CommitmentLevel::Processed)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(program_after.lamports - program_before.lamports, 0);
    }

    #[tokio::test]
    async fn test_change_tip_receiver_pay_block_builder_program() {
        let (config_pubkey, config_bump) =
            Pubkey::find_program_address(&[&CONFIG_ACCOUNT_SEED], &jito_tip_payment::id());
        let tip_pdas = get_tip_pdas();
        let validator_identity = Keypair::new();
        let vote_account = Pubkey::new_unique();

        let tip_receiver = Keypair::new();
        let config = Config {
            tip_receiver: tip_receiver.pubkey(),
            block_builder: jito_tip_distribution::id(),
            block_builder_commission_pct: 50,
            bumps: InitBumps {
                config: config_bump,
                tip_payment_account_0: tip_pdas[0].1,
                tip_payment_account_1: tip_pdas[1].1,
                tip_payment_account_2: tip_pdas[2].1,
                tip_payment_account_3: tip_pdas[3].1,
                tip_payment_account_4: tip_pdas[4].1,
                tip_payment_account_5: tip_pdas[5].1,
                tip_payment_account_6: tip_pdas[6].1,
                tip_payment_account_7: tip_pdas[7].1,
            },
        };
        let mut initial_accounts = get_initial_accounts(config);
        initial_accounts.push((
            vote_account,
            build_fake_vote_account(&validator_identity.pubkey()),
        ));

        let ProgramTestContext {
            banks_client,
            last_blockhash,
            payer,
            ..
        } = get_test(&initial_accounts).await;

        let program_before = banks_client
            .get_account_with_commitment(jito_tip_distribution::id(), CommitmentLevel::Processed)
            .await
            .unwrap()
            .unwrap();

        let epoch = get_epoch(&banks_client).await;
        let new_tip_receiver = get_tip_distribution_pda(&vote_account, epoch);
        banks_client
            .process_transaction_with_commitment(
                Transaction::new_signed_with_payer(
                    &[
                        transfer(&payer.pubkey(), &tip_pdas[1].0, 1_000_000),
                        Instruction {
                            program_id: jito_tip_payment::id(),
                            data: jito_tip_payment::instruction::ChangeTipReceiver {}.data(),
                            accounts: jito_tip_payment::accounts::ChangeTipReceiver {
                                config: config_pubkey,
                                old_tip_receiver: tip_receiver.pubkey(),
                                new_tip_receiver,
                                block_builder: jito_tip_distribution::id(),
                                tip_payment_account_0: tip_pdas[0].0,
                                tip_payment_account_1: tip_pdas[1].0,
                                tip_payment_account_2: tip_pdas[2].0,
                                tip_payment_account_3: tip_pdas[3].0,
                                tip_payment_account_4: tip_pdas[4].0,
                                tip_payment_account_5: tip_pdas[5].0,
                                tip_payment_account_6: tip_pdas[6].0,
                                tip_payment_account_7: tip_pdas[7].0,
                                signer: validator_identity.pubkey(),
                                validator_vote_account: vote_account,
                            }
                            .to_account_metas(None),
                        },
                    ],
                    Some(&payer.pubkey()),
                    &[&payer, &validator_identity],
                    last_blockhash,
                ),
                CommitmentLevel::Processed,
            )
            .await
            .unwrap();

        let program_after = banks_client
            .get_account_with_commitment(jito_tip_distribution::id(), CommitmentLevel::Processed)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(program_after.lamports - program_before.lamports, 0);
    }

    #[tokio::test]
    async fn test_change_block_builder_pay_tip_receiver_program() {
        let (config_pubkey, config_bump) =
            Pubkey::find_program_address(&[&CONFIG_ACCOUNT_SEED], &jito_tip_payment::id());
        let tip_pdas = get_tip_pdas();

        let block_builder = Keypair::new();
        let config = Config {
            tip_receiver: jito_tip_distribution::id(),
            block_builder: block_builder.pubkey(),
            block_builder_commission_pct: 50,
            bumps: InitBumps {
                config: config_bump,
                tip_payment_account_0: tip_pdas[0].1,
                tip_payment_account_1: tip_pdas[1].1,
                tip_payment_account_2: tip_pdas[2].1,
                tip_payment_account_3: tip_pdas[3].1,
                tip_payment_account_4: tip_pdas[4].1,
                tip_payment_account_5: tip_pdas[5].1,
                tip_payment_account_6: tip_pdas[6].1,
                tip_payment_account_7: tip_pdas[7].1,
            },
        };
        let initial_accounts = get_initial_accounts(config);

        let ProgramTestContext {
            banks_client,
            last_blockhash,
            payer,
            ..
        } = get_test(&initial_accounts).await;

        let program_before = banks_client
            .get_account_with_commitment(jito_tip_distribution::id(), CommitmentLevel::Processed)
            .await
            .unwrap()
            .unwrap();

        banks_client
            .process_transaction_with_commitment(
                Transaction::new_signed_with_payer(
                    &[
                        transfer(&payer.pubkey(), &tip_pdas[1].0, 1_000_000),
                        Instruction {
                            program_id: jito_tip_payment::id(),
                            data: jito_tip_payment::instruction::ChangeBlockBuilder {
                                block_builder_commission: 50,
                            }
                            .data(),
                            accounts: jito_tip_payment::accounts::ChangeBlockBuilder {
                                config: config_pubkey,
                                tip_receiver: jito_tip_distribution::id(),
                                old_block_builder: block_builder.pubkey(),
                                new_block_builder: Pubkey::new_unique(),
                                tip_payment_account_0: tip_pdas[0].0,
                                tip_payment_account_1: tip_pdas[1].0,
                                tip_payment_account_2: tip_pdas[2].0,
                                tip_payment_account_3: tip_pdas[3].0,
                                tip_payment_account_4: tip_pdas[4].0,
                                tip_payment_account_5: tip_pdas[5].0,
                                tip_payment_account_6: tip_pdas[6].0,
                                tip_payment_account_7: tip_pdas[7].0,
                                signer: payer.pubkey(),
                            }
                            .to_account_metas(None),
                        },
                    ],
                    Some(&payer.pubkey()),
                    &[&payer],
                    last_blockhash,
                ),
                CommitmentLevel::Processed,
            )
            .await
            .unwrap();

        let program_after = banks_client
            .get_account_with_commitment(jito_tip_distribution::id(), CommitmentLevel::Processed)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(program_after.lamports - program_before.lamports, 0);
    }

    #[tokio::test]
    async fn test_change_block_builder_pay_block_builder_program() {
        let (config_pubkey, config_bump) =
            Pubkey::find_program_address(&[&CONFIG_ACCOUNT_SEED], &jito_tip_payment::id());
        let tip_pdas = get_tip_pdas();

        let tip_receiver = Keypair::new();
        let config = Config {
            tip_receiver: tip_receiver.pubkey(),
            block_builder: jito_tip_distribution::id(),
            block_builder_commission_pct: 50,
            bumps: InitBumps {
                config: config_bump,
                tip_payment_account_0: tip_pdas[0].1,
                tip_payment_account_1: tip_pdas[1].1,
                tip_payment_account_2: tip_pdas[2].1,
                tip_payment_account_3: tip_pdas[3].1,
                tip_payment_account_4: tip_pdas[4].1,
                tip_payment_account_5: tip_pdas[5].1,
                tip_payment_account_6: tip_pdas[6].1,
                tip_payment_account_7: tip_pdas[7].1,
            },
        };
        let initial_accounts = get_initial_accounts(config);

        let ProgramTestContext {
            banks_client,
            last_blockhash,
            payer,
            ..
        } = get_test(&initial_accounts).await;

        let program_before = banks_client
            .get_account_with_commitment(jito_tip_distribution::id(), CommitmentLevel::Processed)
            .await
            .unwrap()
            .unwrap();

        banks_client
            .process_transaction_with_commitment(
                Transaction::new_signed_with_payer(
                    &[
                        transfer(&payer.pubkey(), &tip_pdas[1].0, 1_000_000),
                        Instruction {
                            program_id: jito_tip_payment::id(),
                            data: jito_tip_payment::instruction::ChangeBlockBuilder {
                                block_builder_commission: 50,
                            }
                            .data(),
                            accounts: jito_tip_payment::accounts::ChangeBlockBuilder {
                                config: config_pubkey,
                                tip_receiver: tip_receiver.pubkey(),
                                old_block_builder: jito_tip_distribution::id(),
                                new_block_builder: Pubkey::new_unique(),
                                tip_payment_account_0: tip_pdas[0].0,
                                tip_payment_account_1: tip_pdas[1].0,
                                tip_payment_account_2: tip_pdas[2].0,
                                tip_payment_account_3: tip_pdas[3].0,
                                tip_payment_account_4: tip_pdas[4].0,
                                tip_payment_account_5: tip_pdas[5].0,
                                tip_payment_account_6: tip_pdas[6].0,
                                tip_payment_account_7: tip_pdas[7].0,
                                signer: payer.pubkey(),
                            }
                            .to_account_metas(None),
                        },
                    ],
                    Some(&payer.pubkey()),
                    &[&payer],
                    last_blockhash,
                ),
                CommitmentLevel::Processed,
            )
            .await
            .unwrap();

        let program_after = banks_client
            .get_account_with_commitment(jito_tip_distribution::id(), CommitmentLevel::Processed)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(program_after.lamports - program_before.lamports, 0);
    }

    /// Drift detector: `TIP_DISTRIBUTION_PROGRAM_ID`/`TIP_DISTRIBUTION_ACCOUNT_SEED` are copied
    /// into tip-payment (rather than depending on the jito-tip-distribution crate directly) to
    /// avoid pulling its dependency tree/BPF footprint in. This guards against the two copies
    /// silently diverging.
    #[test]
    fn test_tip_distribution_constants_match_sibling_crate() {
        assert_eq!(TIP_DISTRIBUTION_PROGRAM_ID, jito_tip_distribution::id());
        assert_eq!(
            TIP_DISTRIBUTION_ACCOUNT_SEED,
            jito_tip_distribution::state::TipDistributionAccount::SEED
        );
    }

    #[tokio::test]
    async fn test_change_tip_receiver_ok_with_valid_vote_account() {
        let validator_identity = Keypair::new();
        let vote_account = Pubkey::new_unique();

        let ProgramTestContext {
            mut banks_client,
            last_blockhash,
            payer,
            ..
        } = get_test(&[(
            vote_account,
            build_fake_vote_account(&validator_identity.pubkey()),
        )])
        .await;
        initialize_program(&mut banks_client, &payer, last_blockhash).await;

        let config_pda = get_config_pda();
        let tip_pdas = get_tip_pdas();
        let epoch = get_epoch(&banks_client).await;
        let new_tip_receiver = get_tip_distribution_pda(&vote_account, epoch);

        let change_tip_receiver_ix = Instruction {
            program_id: jito_tip_payment::id(),
            data: jito_tip_payment::instruction::ChangeTipReceiver {}.data(),
            accounts: jito_tip_payment::accounts::ChangeTipReceiver {
                config: config_pda.0,
                old_tip_receiver: payer.pubkey(),
                new_tip_receiver,
                block_builder: payer.pubkey(),
                tip_payment_account_0: tip_pdas[0].0,
                tip_payment_account_1: tip_pdas[1].0,
                tip_payment_account_2: tip_pdas[2].0,
                tip_payment_account_3: tip_pdas[3].0,
                tip_payment_account_4: tip_pdas[4].0,
                tip_payment_account_5: tip_pdas[5].0,
                tip_payment_account_6: tip_pdas[6].0,
                tip_payment_account_7: tip_pdas[7].0,
                signer: validator_identity.pubkey(),
                validator_vote_account: vote_account,
            }
            .to_account_metas(None),
        };
        let tx = Transaction::new_signed_with_payer(
            &[change_tip_receiver_ix],
            Some(&payer.pubkey()),
            &[&payer, &validator_identity],
            last_blockhash,
        );
        banks_client
            .process_transaction_with_commitment(tx, CommitmentLevel::Processed)
            .await
            .unwrap();

        let config_account = banks_client
            .get_account_with_commitment(config_pda.0, CommitmentLevel::Processed)
            .await
            .unwrap()
            .unwrap();
        let config_account = Config::try_deserialize(&mut config_account.data()).unwrap();
        assert_eq!(config_account.tip_receiver, new_tip_receiver);
    }

    #[tokio::test]
    async fn test_change_tip_receiver_signer_not_node_pubkey_fails() {
        let validator_identity = Keypair::new();
        let vote_account = Pubkey::new_unique();

        let ProgramTestContext {
            mut banks_client,
            last_blockhash,
            payer,
            ..
        } = get_test(&[(
            vote_account,
            build_fake_vote_account(&validator_identity.pubkey()),
        )])
        .await;
        initialize_program(&mut banks_client, &payer, last_blockhash).await;

        let config_pda = get_config_pda();
        let tip_pdas = get_tip_pdas();
        let epoch = get_epoch(&banks_client).await;
        let new_tip_receiver = get_tip_distribution_pda(&vote_account, epoch);

        // `payer` signs (and is a valid Signer for the ix), but the vote account's node_pubkey
        // is `validator_identity`, not `payer` — the identity check must reject this.
        let change_tip_receiver_ix = Instruction {
            program_id: jito_tip_payment::id(),
            data: jito_tip_payment::instruction::ChangeTipReceiver {}.data(),
            accounts: jito_tip_payment::accounts::ChangeTipReceiver {
                config: config_pda.0,
                old_tip_receiver: payer.pubkey(),
                new_tip_receiver,
                block_builder: payer.pubkey(),
                tip_payment_account_0: tip_pdas[0].0,
                tip_payment_account_1: tip_pdas[1].0,
                tip_payment_account_2: tip_pdas[2].0,
                tip_payment_account_3: tip_pdas[3].0,
                tip_payment_account_4: tip_pdas[4].0,
                tip_payment_account_5: tip_pdas[5].0,
                tip_payment_account_6: tip_pdas[6].0,
                tip_payment_account_7: tip_pdas[7].0,
                signer: payer.pubkey(),
                validator_vote_account: vote_account,
            }
            .to_account_metas(None),
        };
        let tx = Transaction::new_signed_with_payer(
            &[change_tip_receiver_ix],
            Some(&payer.pubkey()),
            &[&payer],
            last_blockhash,
        );
        let err = banks_client.process_transaction(tx).await.unwrap_err();
        assert_eq!(
            err.unwrap(),
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    ERROR_CODE_OFFSET + TipPaymentError::InvalidValidatorVoteAccount as u32
                )
            )
        );
    }

    #[tokio::test]
    async fn test_change_tip_receiver_wrong_pda_fails() {
        let validator_identity = Keypair::new();
        let vote_account = Pubkey::new_unique();

        let ProgramTestContext {
            mut banks_client,
            last_blockhash,
            payer,
            ..
        } = get_test(&[(
            vote_account,
            build_fake_vote_account(&validator_identity.pubkey()),
        )])
        .await;
        initialize_program(&mut banks_client, &payer, last_blockhash).await;

        let config_pda = get_config_pda();
        let tip_pdas = get_tip_pdas();

        // signer/vote_account are both valid, but new_tip_receiver isn't the derived PDA.
        let change_tip_receiver_ix = Instruction {
            program_id: jito_tip_payment::id(),
            data: jito_tip_payment::instruction::ChangeTipReceiver {}.data(),
            accounts: jito_tip_payment::accounts::ChangeTipReceiver {
                config: config_pda.0,
                old_tip_receiver: payer.pubkey(),
                new_tip_receiver: Pubkey::new_unique(),
                block_builder: payer.pubkey(),
                tip_payment_account_0: tip_pdas[0].0,
                tip_payment_account_1: tip_pdas[1].0,
                tip_payment_account_2: tip_pdas[2].0,
                tip_payment_account_3: tip_pdas[3].0,
                tip_payment_account_4: tip_pdas[4].0,
                tip_payment_account_5: tip_pdas[5].0,
                tip_payment_account_6: tip_pdas[6].0,
                tip_payment_account_7: tip_pdas[7].0,
                signer: validator_identity.pubkey(),
                validator_vote_account: vote_account,
            }
            .to_account_metas(None),
        };
        let tx = Transaction::new_signed_with_payer(
            &[change_tip_receiver_ix],
            Some(&payer.pubkey()),
            &[&payer, &validator_identity],
            last_blockhash,
        );
        let err = banks_client.process_transaction(tx).await.unwrap_err();
        assert_eq!(
            err.unwrap(),
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(
                    ERROR_CODE_OFFSET + TipPaymentError::TipReceiverPdaMismatch as u32
                )
            )
        );
    }

    #[tokio::test]
    async fn test_change_tip_receiver_vote_account_wrong_owner_fails() {
        let ProgramTestContext {
            mut banks_client,
            last_blockhash,
            payer,
            ..
        } = get_test(&[]).await;
        initialize_program(&mut banks_client, &payer, last_blockhash).await;

        let config_pda = get_config_pda();
        let tip_pdas = get_tip_pdas();

        // `payer.pubkey()` is a real, funded account, but it's owned by the System program, not
        // the vote program — deserialize_node_pubkey must reject it before any identity/PDA
        // comparison happens.
        let change_tip_receiver_ix = Instruction {
            program_id: jito_tip_payment::id(),
            data: jito_tip_payment::instruction::ChangeTipReceiver {}.data(),
            accounts: jito_tip_payment::accounts::ChangeTipReceiver {
                config: config_pda.0,
                old_tip_receiver: payer.pubkey(),
                new_tip_receiver: Pubkey::new_unique(),
                block_builder: payer.pubkey(),
                tip_payment_account_0: tip_pdas[0].0,
                tip_payment_account_1: tip_pdas[1].0,
                tip_payment_account_2: tip_pdas[2].0,
                tip_payment_account_3: tip_pdas[3].0,
                tip_payment_account_4: tip_pdas[4].0,
                tip_payment_account_5: tip_pdas[5].0,
                tip_payment_account_6: tip_pdas[6].0,
                tip_payment_account_7: tip_pdas[7].0,
                signer: payer.pubkey(),
                validator_vote_account: payer.pubkey(),
            }
            .to_account_metas(None),
        };
        let tx = Transaction::new_signed_with_payer(
            &[change_tip_receiver_ix],
            Some(&payer.pubkey()),
            &[&payer],
            last_blockhash,
        );
        let err = banks_client.process_transaction(tx).await.unwrap_err();
        assert_eq!(
            err.unwrap(),
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(ErrorCode::ConstraintOwner as u32)
            )
        );
    }

    /// Regression: a correctly-derived TDA PDA that has not yet been created on-chain (0
    /// lamports, owner = System) must still be accepted — honest validators routinely rotate
    /// to a TDA before `initialize_tip_distribution_account` has run for the new epoch.
    #[tokio::test]
    async fn test_change_tip_receiver_tda_not_yet_created_ok() {
        let validator_identity = Keypair::new();
        let vote_account = Pubkey::new_unique();

        let ProgramTestContext {
            mut banks_client,
            last_blockhash,
            payer,
            ..
        } = get_test(&[(
            vote_account,
            build_fake_vote_account(&validator_identity.pubkey()),
        )])
        .await;
        initialize_program(&mut banks_client, &payer, last_blockhash).await;

        let config_pda = get_config_pda();
        let tip_pdas = get_tip_pdas();
        let epoch = get_epoch(&banks_client).await;
        let new_tip_receiver = get_tip_distribution_pda(&vote_account, epoch);

        assert!(banks_client
            .get_account_with_commitment(new_tip_receiver, CommitmentLevel::Processed)
            .await
            .unwrap()
            .is_none());

        let change_tip_receiver_ix = Instruction {
            program_id: jito_tip_payment::id(),
            data: jito_tip_payment::instruction::ChangeTipReceiver {}.data(),
            accounts: jito_tip_payment::accounts::ChangeTipReceiver {
                config: config_pda.0,
                old_tip_receiver: payer.pubkey(),
                new_tip_receiver,
                block_builder: payer.pubkey(),
                tip_payment_account_0: tip_pdas[0].0,
                tip_payment_account_1: tip_pdas[1].0,
                tip_payment_account_2: tip_pdas[2].0,
                tip_payment_account_3: tip_pdas[3].0,
                tip_payment_account_4: tip_pdas[4].0,
                tip_payment_account_5: tip_pdas[5].0,
                tip_payment_account_6: tip_pdas[6].0,
                tip_payment_account_7: tip_pdas[7].0,
                signer: validator_identity.pubkey(),
                validator_vote_account: vote_account,
            }
            .to_account_metas(None),
        };
        let tx = Transaction::new_signed_with_payer(
            &[change_tip_receiver_ix],
            Some(&payer.pubkey()),
            &[&payer, &validator_identity],
            last_blockhash,
        );
        banks_client
            .process_transaction_with_commitment(tx, CommitmentLevel::Processed)
            .await
            .unwrap();

        let config_account = banks_client
            .get_account_with_commitment(config_pda.0, CommitmentLevel::Processed)
            .await
            .unwrap()
            .unwrap();
        let config_account = Config::try_deserialize(&mut config_account.data()).unwrap();
        assert_eq!(config_account.tip_receiver, new_tip_receiver);

        // still not created — change_tip_receiver never touches TDA existence.
        assert!(banks_client
            .get_account_with_commitment(new_tip_receiver, CommitmentLevel::Processed)
            .await
            .unwrap()
            .is_none());
    }

    /// This is the behavior an upgraded program shows an old, not-yet-upgraded client: a clean
    /// typed error, not a panic (per the plan's upgrade-ordering safety argument).
    #[tokio::test]
    async fn test_change_tip_receiver_missing_vote_account_rejected_cleanly() {
        let ProgramTestContext {
            mut banks_client,
            last_blockhash,
            payer,
            ..
        } = get_test(&[]).await;
        initialize_program(&mut banks_client, &payer, last_blockhash).await;

        let config_pda = get_config_pda();
        let tip_pdas = get_tip_pdas();

        let mut metas = jito_tip_payment::accounts::ChangeTipReceiver {
            config: config_pda.0,
            old_tip_receiver: payer.pubkey(),
            new_tip_receiver: Pubkey::new_unique(),
            block_builder: payer.pubkey(),
            tip_payment_account_0: tip_pdas[0].0,
            tip_payment_account_1: tip_pdas[1].0,
            tip_payment_account_2: tip_pdas[2].0,
            tip_payment_account_3: tip_pdas[3].0,
            tip_payment_account_4: tip_pdas[4].0,
            tip_payment_account_5: tip_pdas[5].0,
            tip_payment_account_6: tip_pdas[6].0,
            tip_payment_account_7: tip_pdas[7].0,
            signer: payer.pubkey(),
            validator_vote_account: Pubkey::new_unique(),
        }
        .to_account_metas(None);
        // simulate an old client shape: drop the trailing validator_vote_account account.
        metas.pop();

        let change_tip_receiver_ix = Instruction {
            program_id: jito_tip_payment::id(),
            data: jito_tip_payment::instruction::ChangeTipReceiver {}.data(),
            accounts: metas,
        };
        let tx = Transaction::new_signed_with_payer(
            &[change_tip_receiver_ix],
            Some(&payer.pubkey()),
            &[&payer],
            last_blockhash,
        );
        let err = banks_client.process_transaction(tx).await.unwrap_err();
        assert_eq!(
            err.unwrap(),
            TransactionError::InstructionError(
                0,
                InstructionError::Custom(ErrorCode::AccountNotEnoughKeys as u32)
            )
        );
    }

    /// Real evidence for the two-phase rollout's safety assumption: extra trailing accounts
    /// beyond what a program's `Accounts` struct declares fall into `remaining_accounts` and
    /// are silently ignored rather than erroring. We can't literally deploy the pre-upgrade
    /// bytecode in this test binary (only one program version is compiled in), so this proves
    /// the general Anchor mechanism using one extra account beyond what *this* program's struct
    /// declares, standing in for "one more account than an older program's struct declares."
    #[tokio::test]
    async fn test_change_tip_receiver_tolerates_extra_trailing_account() {
        let validator_identity = Keypair::new();
        let vote_account = Pubkey::new_unique();

        let ProgramTestContext {
            mut banks_client,
            last_blockhash,
            payer,
            ..
        } = get_test(&[(
            vote_account,
            build_fake_vote_account(&validator_identity.pubkey()),
        )])
        .await;
        initialize_program(&mut banks_client, &payer, last_blockhash).await;

        let config_pda = get_config_pda();
        let tip_pdas = get_tip_pdas();
        let epoch = get_epoch(&banks_client).await;
        let new_tip_receiver = get_tip_distribution_pda(&vote_account, epoch);

        let mut metas = jito_tip_payment::accounts::ChangeTipReceiver {
            config: config_pda.0,
            old_tip_receiver: payer.pubkey(),
            new_tip_receiver,
            block_builder: payer.pubkey(),
            tip_payment_account_0: tip_pdas[0].0,
            tip_payment_account_1: tip_pdas[1].0,
            tip_payment_account_2: tip_pdas[2].0,
            tip_payment_account_3: tip_pdas[3].0,
            tip_payment_account_4: tip_pdas[4].0,
            tip_payment_account_5: tip_pdas[5].0,
            tip_payment_account_6: tip_pdas[6].0,
            tip_payment_account_7: tip_pdas[7].0,
            signer: validator_identity.pubkey(),
            validator_vote_account: vote_account,
        }
        .to_account_metas(None);
        metas.push(AccountMeta::new_readonly(Pubkey::new_unique(), false));

        let change_tip_receiver_ix = Instruction {
            program_id: jito_tip_payment::id(),
            data: jito_tip_payment::instruction::ChangeTipReceiver {}.data(),
            accounts: metas,
        };
        let tx = Transaction::new_signed_with_payer(
            &[change_tip_receiver_ix],
            Some(&payer.pubkey()),
            &[&payer, &validator_identity],
            last_blockhash,
        );
        banks_client
            .process_transaction_with_commitment(tx, CommitmentLevel::Processed)
            .await
            .unwrap();

        let config_account = banks_client
            .get_account_with_commitment(config_pda.0, CommitmentLevel::Processed)
            .await
            .unwrap()
            .unwrap();
        let config_account = Config::try_deserialize(&mut config_account.data()).unwrap();
        assert_eq!(config_account.tip_receiver, new_tip_receiver);
    }

    /// Regression: `change_block_builder`'s accounts/behavior are completely untouched by this
    /// fix — it has no vote-account binding and isn't supposed to (see plan doc's TODO on the
    /// twin `change_block_builder` bug, handled separately).
    #[tokio::test]
    async fn test_change_block_builder_unaffected_by_vote_account_changes() {
        let ProgramTestContext {
            mut banks_client,
            last_blockhash,
            payer,
            ..
        } = get_test(&[]).await;
        initialize_program(&mut banks_client, &payer, last_blockhash).await;

        let config_pda = get_config_pda();
        let tip_pdas = get_tip_pdas();
        let new_block_builder = Pubkey::new_unique();

        let change_block_builder_ix = Instruction {
            program_id: jito_tip_payment::id(),
            data: jito_tip_payment::instruction::ChangeBlockBuilder {
                block_builder_commission: 0,
            }
            .data(),
            accounts: jito_tip_payment::accounts::ChangeBlockBuilder {
                config: config_pda.0,
                tip_receiver: payer.pubkey(),
                old_block_builder: payer.pubkey(),
                new_block_builder,
                tip_payment_account_0: tip_pdas[0].0,
                tip_payment_account_1: tip_pdas[1].0,
                tip_payment_account_2: tip_pdas[2].0,
                tip_payment_account_3: tip_pdas[3].0,
                tip_payment_account_4: tip_pdas[4].0,
                tip_payment_account_5: tip_pdas[5].0,
                tip_payment_account_6: tip_pdas[6].0,
                tip_payment_account_7: tip_pdas[7].0,
                signer: payer.pubkey(),
            }
            .to_account_metas(None),
        };
        let tx = Transaction::new_signed_with_payer(
            &[change_block_builder_ix],
            Some(&payer.pubkey()),
            &[&payer],
            last_blockhash,
        );
        banks_client
            .process_transaction_with_commitment(tx, CommitmentLevel::Processed)
            .await
            .unwrap();

        let config_account = banks_client
            .get_account_with_commitment(config_pda.0, CommitmentLevel::Processed)
            .await
            .unwrap()
            .unwrap();
        let config_account = Config::try_deserialize(&mut config_account.data()).unwrap();
        assert_eq!(config_account.block_builder, new_block_builder);
    }

    /// Documents an accepted residual risk (see plan doc's "残余风险" section): the identity
    /// check only proves the signer controls *some* vote account's node_pubkey, not that the
    /// vote account is real/staked/in the leader schedule. A throwaway, freshly-created,
    /// zero-stake vote account with an attacker-controlled node_pubkey passes validation just
    /// as well as a real one. This is intentionally NOT fixed on-chain (Solana has no cheap way
    /// to check a vote account's stake or leader-schedule membership) — it's mitigated by
    /// off-chain monitoring instead. This test exists so nobody "fixes" this path later without
    /// re-opening that discussion.
    #[tokio::test]
    async fn test_change_tip_receiver_throwaway_vote_account_residual_risk() {
        let attacker = Keypair::new();
        let throwaway_vote_account = Pubkey::new_unique();

        let ProgramTestContext {
            mut banks_client,
            last_blockhash,
            payer,
            ..
        } = get_test(&[(
            throwaway_vote_account,
            build_fake_vote_account(&attacker.pubkey()),
        )])
        .await;
        initialize_program(&mut banks_client, &payer, last_blockhash).await;

        let config_pda = get_config_pda();
        let tip_pdas = get_tip_pdas();
        let epoch = get_epoch(&banks_client).await;
        let attacker_tip_receiver = get_tip_distribution_pda(&throwaway_vote_account, epoch);

        let change_tip_receiver_ix = Instruction {
            program_id: jito_tip_payment::id(),
            data: jito_tip_payment::instruction::ChangeTipReceiver {}.data(),
            accounts: jito_tip_payment::accounts::ChangeTipReceiver {
                config: config_pda.0,
                old_tip_receiver: payer.pubkey(),
                new_tip_receiver: attacker_tip_receiver,
                block_builder: payer.pubkey(),
                tip_payment_account_0: tip_pdas[0].0,
                tip_payment_account_1: tip_pdas[1].0,
                tip_payment_account_2: tip_pdas[2].0,
                tip_payment_account_3: tip_pdas[3].0,
                tip_payment_account_4: tip_pdas[4].0,
                tip_payment_account_5: tip_pdas[5].0,
                tip_payment_account_6: tip_pdas[6].0,
                tip_payment_account_7: tip_pdas[7].0,
                signer: attacker.pubkey(),
                validator_vote_account: throwaway_vote_account,
            }
            .to_account_metas(None),
        };
        let tx = Transaction::new_signed_with_payer(
            &[change_tip_receiver_ix],
            Some(&payer.pubkey()),
            &[&payer, &attacker],
            last_blockhash,
        );
        // accepted residual risk: this succeeds today, on-chain checks alone can't close it.
        banks_client
            .process_transaction_with_commitment(tx, CommitmentLevel::Processed)
            .await
            .unwrap();

        let config_account = banks_client
            .get_account_with_commitment(config_pda.0, CommitmentLevel::Processed)
            .await
            .unwrap()
            .unwrap();
        let config_account = Config::try_deserialize(&mut config_account.data()).unwrap();
        assert_eq!(config_account.tip_receiver, attacker_tip_receiver);
    }
}
