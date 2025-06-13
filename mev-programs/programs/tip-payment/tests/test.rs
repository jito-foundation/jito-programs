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
    };
    use solana_program_test::{BanksClient, ProgramTest, ProgramTestContext};
    use solana_sdk::{
        account::{Account, ReadableAccount},
        bpf_loader_upgradeable,
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
        let ProgramTestContext {
            mut banks_client,
            last_blockhash,
            payer,
            ..
        } = get_test(&[]).await;
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
                signer: payer.pubkey(),
            }
            .to_account_metas(None),
        };
        change_tip_receiver_ix
            .accounts
            .push(AccountMeta::new(bpf_loader_upgradeable::id(), false));
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
                    ERROR_CODE_OFFSET + TipPaymentError::InvalidTipReceiver as u32
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
        let ProgramTestContext {
            mut banks_client,
            last_blockhash,
            payer,
            ..
        } = get_test(&[]).await;
        initialize_program(&mut banks_client, &payer, last_blockhash).await;

        let config_pda = get_config_pda();
        let tip_pdas = get_tip_pdas();

        // set the tip receiver and transfer 1 lamport to the tip pda 1
        let new_tip_receiver = Pubkey::new_unique();
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
        let ProgramTestContext {
            mut banks_client,
            last_blockhash,
            payer,
            ..
        } = get_test(&[]).await;
        initialize_program(&mut banks_client, &payer, last_blockhash).await;

        let config_pda = get_config_pda();
        let tip_pdas = get_tip_pdas();

        // set the tip receiver and transfer 1 lamport to the tip pda 1
        let new_tip_receiver = Pubkey::new_unique();
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

        banks_client
            .process_transaction_with_commitment(
                Transaction::new_signed_with_payer(
                    &[Instruction {
                        program_id: jito_tip_payment::id(),
                        data: jito_tip_payment::instruction::ChangeTipReceiver {}.data(),
                        accounts: jito_tip_payment::accounts::ChangeTipReceiver {
                            config: config_pda.0,
                            old_tip_receiver: new_tip_receiver,
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
    async fn test_change_tip_receiver_below_rent_exempt_block_builder() {
        let ProgramTestContext {
            mut banks_client,
            last_blockhash,
            payer,
            ..
        } = get_test(&[]).await;
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

        banks_client
            .process_transaction_with_commitment(
                Transaction::new_signed_with_payer(
                    &[Instruction {
                        program_id: jito_tip_payment::id(),
                        data: jito_tip_payment::instruction::ChangeTipReceiver {}.data(),
                        accounts: jito_tip_payment::accounts::ChangeTipReceiver {
                            config: config_pda.0,
                            old_tip_receiver: payer.pubkey(),
                            new_tip_receiver: Pubkey::new_unique(),
                            block_builder: new_block_builder,
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
        let ProgramTestContext {
            mut banks_client,
            last_blockhash,
            payer,
            ..
        } = get_test(&[]).await;
        initialize_program(&mut banks_client, &payer, last_blockhash).await;

        let config_pda = get_config_pda();
        let tip_pdas = get_tip_pdas();

        // set the tip receiver and transfer 1 lamport to the tip pda 1
        let new_block_builder = Pubkey::new_unique();
        let new_tip_receiver = Pubkey::new_unique();
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
                                signer: payer.pubkey(),
                            }
                            .to_account_metas(None),
                        },
                        transfer(&payer.pubkey(), &tip_pdas[1].0, 333_333_333),
                    ],
                    Some(&payer.pubkey()),
                    &[&payer],
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
                            data: jito_tip_payment::instruction::ChangeTipReceiver {}.data(),
                            accounts: jito_tip_payment::accounts::ChangeTipReceiver {
                                config: config_pubkey,
                                old_tip_receiver: jito_tip_distribution::id(),
                                new_tip_receiver: Pubkey::new_unique(),
                                block_builder: block_builder.pubkey(),
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
    async fn test_change_tip_receiver_pay_block_builder_program() {
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
                            data: jito_tip_payment::instruction::ChangeTipReceiver {}.data(),
                            accounts: jito_tip_payment::accounts::ChangeTipReceiver {
                                config: config_pubkey,
                                old_tip_receiver: tip_receiver.pubkey(),
                                new_tip_receiver: Pubkey::new_unique(),
                                block_builder: jito_tip_distribution::id(),
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
}
