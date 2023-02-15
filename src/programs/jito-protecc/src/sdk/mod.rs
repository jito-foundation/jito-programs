use anchor_lang::{prelude::*, solana_program::instruction::Instruction, InstructionData};

pub struct PreTokenGuardArgs {
    pub bump: u8,
}
pub struct PreTokenGuardAccounts {
    pub token_guarded_state: Pubkey,
    pub signer: Pubkey,
    pub system_program: Pubkey,
    pub token_account: Pubkey,
}
pub fn pre_token_guard_ix(
    program_id: Pubkey,
    args: PreTokenGuardArgs,
    accounts: PreTokenGuardAccounts,
) -> Instruction {
    let PreTokenGuardArgs { bump } = args;

    let PreTokenGuardAccounts {
        token_guarded_state,
        signer,
        system_program,
        token_account,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::PreTokenGuard { bump }.data(),
        accounts: crate::accounts::PreTokenGuard {
            token_guarded_state,
            signer,
            system_program,
            token_account,
        }
        .to_account_metas(None),
    }
}

pub struct PostTokenGuardArgs;
pub struct PostTokenGuardAccounts {
    pub token_guarded_state: Pubkey,
    pub signer: Pubkey,
    pub token_account: Pubkey,
}
pub fn post_token_guard_ix(
    program_id: Pubkey,
    _args: PostTokenGuardArgs,
    accounts: PostTokenGuardAccounts,
) -> Instruction {
    let PostTokenGuardAccounts {
        token_guarded_state,
        signer,
        token_account,
    } = accounts;

    Instruction {
        program_id,
        data: vec![],
        accounts: crate::accounts::PostTokenGuard {
            token_guarded_state,
            signer,
            token_account,
        }
        .to_account_metas(None),
    }
}

pub struct CloseTokenGuardedStateArgs;
pub struct CloseTokenGuardedStateAccounts {
    pub token_guarded_state: Pubkey,
    pub signer: Pubkey,
    pub token_account: Pubkey,
}
pub fn close_token_guarded_state_ix(
    program_id: Pubkey,
    _args: CloseTokenGuardedStateArgs,
    accounts: CloseTokenGuardedStateAccounts,
) -> Instruction {
    let CloseTokenGuardedStateAccounts {
        token_guarded_state,
        signer,
        token_account,
    } = accounts;

    Instruction {
        program_id,
        data: vec![],
        accounts: crate::accounts::CloseTokenGuardedState {
            token_guarded_state,
            signer,
            token_account,
        }
        .to_account_metas(None),
    }
}

pub struct PreSolGuardArgs {
    pub bump: u8,
}
pub struct PreSolGuardAccounts {
    pub sol_guarded_state: Pubkey,
    pub signer: Pubkey,
    pub system_program: Pubkey,
    pub guarded_account: Pubkey,
}
pub fn pre_sol_guard_ix(
    program_id: Pubkey,
    args: PreSolGuardArgs,
    accounts: PreSolGuardAccounts,
) -> Instruction {
    let PreSolGuardArgs { bump } = args;

    let PreSolGuardAccounts {
        sol_guarded_state,
        signer,
        system_program,
        guarded_account,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::PreSolGuard { bump }.data(),
        accounts: crate::accounts::PreSolGuard {
            sol_guarded_state,
            signer,
            system_program,
            guarded_account,
        }
        .to_account_metas(None),
    }
}

pub struct PostSolGuardArgs;
pub struct PostSolGuardAccounts {
    pub sol_guarded_state: Pubkey,
    pub signer: Pubkey,
    pub guarded_account: Pubkey,
}
pub fn post_sol_guard_ix(
    program_id: Pubkey,
    _args: PostSolGuardArgs,
    accounts: PostSolGuardAccounts,
) -> Instruction {
    let PostSolGuardAccounts {
        sol_guarded_state,
        signer,
        guarded_account,
    } = accounts;

    Instruction {
        program_id,
        data: vec![],
        accounts: crate::accounts::PostSolGuard {
            sol_guarded_state,
            signer,
            guarded_account,
        }
        .to_account_metas(None),
    }
}

pub struct CloseSolGuardedStateArgs;
pub struct CloseSolGuardedStateAccounts {
    pub sol_guarded_state: Pubkey,
    pub signer: Pubkey,
    pub guarded_account: Pubkey,
}
pub fn close_sol_guarded_state_ix(
    program_id: Pubkey,
    _args: CloseSolGuardedStateArgs,
    accounts: CloseSolGuardedStateAccounts,
) -> Instruction {
    let CloseSolGuardedStateAccounts {
        sol_guarded_state,
        signer,
        guarded_account,
    } = accounts;

    Instruction {
        program_id,
        data: vec![],
        accounts: crate::accounts::CloseSolGuardedState {
            sol_guarded_state,
            signer,
            guarded_account,
        }
        .to_account_metas(None),
    }
}
