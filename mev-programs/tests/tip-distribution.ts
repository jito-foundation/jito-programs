import { u64 } from "@saberhq/token-utils";
import * as anchor from "@coral-xyz/anchor";
import { AnchorError, Program } from "@coral-xyz/anchor";
import { JitoTipDistribution } from "../target/types/jito_tip_distribution";
import { assert, expect } from "chai";
import {PublicKey, VoteInit, VoteProgram} from "@solana/web3.js";
import { MerkleTree } from "./merkle-tree";

const {
  SystemProgram,
  sendAndConfirmTransaction,
  LAMPORTS_PER_SOL,
} = anchor.web3;
const CONFIG_ACCOUNT_SEED = "CONFIG_ACCOUNT";
const TIP_DISTRIBUTION_ACCOUNT_LEN = 168;
const CLAIM_STATUS_SEED = "CLAIM_STATUS";
const CLAIM_STATUS_LEN = 104;

const provider = anchor.AnchorProvider.local("http://127.0.0.1:8899", {
  commitment: "confirmed",
  preflightCommitment: "confirmed",
});
anchor.setProvider(provider);

const tipDistribution = anchor.workspace.JitoTipDistribution as Program<JitoTipDistribution>;

// globals
let configAccount, configBump;

describe("tests tip_distribution", () => {
  before(async () => {
    const [acc, bump] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from(CONFIG_ACCOUNT_SEED, "utf8")],
      tipDistribution.programId
    );
    configAccount = acc;
    configBump = bump;
  });

  it("#initialize happy path", async () => {
    // given
    const initializer = await generateAccount(100000000000000);
    const authority = await generateAccount(100000000000000);
    const expiredFundsAccount = await generateAccount(100000000000000);
    const numEpochsValid = new anchor.BN(3);
    const maxValidatorCommissionBps = 1000;

    // then
    try {
      await tipDistribution.rpc.initialize(
        authority.publicKey,
        expiredFundsAccount.publicKey,
        numEpochsValid,
        maxValidatorCommissionBps,
        configBump,
        {
          accounts: {
            config: configAccount,
            systemProgram: SystemProgram.programId,
            initializer: initializer.publicKey,
          },
          signers: [initializer],
        }
      );
    } catch (e) {
      assert.fail("unexpected error: " + e);
    }

    // expect
    const actualConfig = await tipDistribution.account.config.fetch(
      configAccount
    );
    const expected = {
      authority: authority.publicKey,
      expiredFundsAccount: expiredFundsAccount.publicKey,
      numEpochsValid,
      maxValidatorCommissionBps,
    };
    assertConfigState(actualConfig, expected);
  });

  it("#init_tip_distribution_account happy path", async () => {
    // given
    const {
      validatorVoteAccount,
      validatorIdentityKeypair,
      maxValidatorCommissionBps: validatorCommissionBps,
      tipDistributionAccount,
      epochInfo,
      bump,
    } = await setup_initTipDistributionAccount();

    // then
    try {
      await call_initTipDistributionAccount({
        merkleRootUploadAuthority: validatorVoteAccount.publicKey,
        validatorCommissionBps,
        config: configAccount,
        systemProgram: SystemProgram.programId,
        validatorVoteAccount,
        validatorIdentityKeypair,
        tipDistributionAccount,
        bump,
      });
    } catch (e) {
      assert.fail("unexpected error: " + e);
    }

    // expect
    const actual = await tipDistribution.account.tipDistributionAccount.fetch(
      tipDistributionAccount
    );
    const expected = {
      validatorVoteAccount: validatorVoteAccount.publicKey,
      epochCreatedAt: epochInfo.epoch,
      merkleRoot: undefined,
      merkleRootUploadAuthority: validatorVoteAccount.publicKey,
      validatorCommissionBps,
    };
    assertDistributionAccount(actual, expected);
  });

  it("#init_tip_distribution_account fails with [ErrorCode::InvalidValidatorCommissionFeeBps]", async () => {
    // given
    const {
      validatorVoteAccount,
      maxValidatorCommissionBps,
      validatorIdentityKeypair,
      tipDistributionAccount,
      bump,
    } = await setup_initTipDistributionAccount();

    // then
    try {
      await call_initTipDistributionAccount({
        validatorCommissionBps: maxValidatorCommissionBps + 1,
        merkleRootUploadAuthority: validatorVoteAccount.publicKey,
        config: configAccount,
        validatorIdentityKeypair,
        systemProgram: SystemProgram.programId,
        validatorVoteAccount,
        tipDistributionAccount,
        bump,
      });
      assert.fail("expected exception to be thrown");
    } catch (e) {
      // expect
      assert(
        e.errorLogs[0].includes(
          "Validator's commission basis points must be less than or equal to the Config account's max_validator_commission_bps."
        )
      );
    }
  });

  it("#close_tip_distribution_account happy path", async () => {
    // given
    const {
      validatorVoteAccount,
      maxValidatorCommissionBps: validatorCommissionBps,
      tipDistributionAccount,
      validatorIdentityKeypair,
      bump,
    } = await setup_initTipDistributionAccount();

    await call_initTipDistributionAccount({
      merkleRootUploadAuthority: validatorVoteAccount.publicKey,
      validatorCommissionBps,
      validatorIdentityKeypair,
      config: configAccount,
      systemProgram: SystemProgram.programId,
      validatorVoteAccount,
      tipDistributionAccount,
      bump,
    });

    const actualConfig = await tipDistribution.account.config.fetch(
      configAccount
    );
    const tda = await tipDistribution.account.tipDistributionAccount.fetch(
      tipDistributionAccount
    );

    const balStart = await provider.connection.getBalance(
      validatorVoteAccount.publicKey
    );
    await sleepForEpochs(4);

    //close the account
    await tipDistribution.methods
      .closeTipDistributionAccount(tda.epochCreatedAt)
      .accounts({
        config: configAccount,
        tipDistributionAccount,
        expiredFundsAccount: actualConfig.expiredFundsAccount,
        validatorVoteAccount: validatorVoteAccount.publicKey, //funds transferred to this account
      })
      .rpc();

    const balEnd = await provider.connection.getBalance(
      validatorVoteAccount.publicKey
    );

    const minRentExempt =
      await provider.connection.getMinimumBalanceForRentExemption(
        TIP_DISTRIBUTION_ACCOUNT_LEN
      );
    assert(balEnd - balStart === minRentExempt);

    try {
      // cannot fetch a closed account
      await tipDistribution.account.tipDistributionAccount.fetch(
        tipDistributionAccount
      );
      assert.fail("fetch should fail");
    } catch (_err) {
      const err: Error = _err;
      expect(err.message).to.contain("Account does not exist");
    }
  });

  it("#upload_merkle_root happy path", async () => {
    const {
      validatorVoteAccount,
      maxValidatorCommissionBps,
      tipDistributionAccount,
      validatorIdentityKeypair,
      epochInfo,
      bump,
    } = await setup_initTipDistributionAccount();
    await call_initTipDistributionAccount({
      validatorCommissionBps: maxValidatorCommissionBps,
      config: configAccount,
      validatorIdentityKeypair,
      systemProgram: SystemProgram.programId,
      merkleRootUploadAuthority: validatorVoteAccount.publicKey,
      validatorVoteAccount,
      tipDistributionAccount,
      bump,
    });

    const user0 = await generateAccount(1000000);
    const user1 = await generateAccount(1000000);
    const amount0 = new u64(1_000_000);
    const amount1 = new u64(2_000_000);
    const demoData = [
      { account: user0.publicKey, amount: new u64(amount0) },
      { account: user1.publicKey, amount: new u64(amount1) },
    ].map(({ account, amount }) => balanceToBuffer(account, amount));

    const tree = new MerkleTree(demoData);

    const root = tree.getRoot();
    const maxTotalClaim = new anchor.BN(amount0.add(amount1));
    const maxNumNodes = new anchor.BN(2);

    await sleepForEpochs(1);
    try {
      await tipDistribution.rpc.uploadMerkleRoot(
        root.toJSON().data,
        maxTotalClaim,
        maxNumNodes,
        {
          accounts: {
            tipDistributionAccount,
            merkleRootUploadAuthority: validatorVoteAccount.publicKey,
            config: configAccount,
          },
          signers: [validatorVoteAccount],
        }
      );
    } catch (e) {
      assert.fail("Unexpected error: " + e);
    }

    const actual = await tipDistribution.account.tipDistributionAccount.fetch(
      tipDistributionAccount
    );
    const expected = {
      validatorVoteAccount: validatorVoteAccount.publicKey,
      epochCreatedAt: epochInfo.epoch,
      merkleRoot: {
        root: [...root],
        maxTotalClaim,
        maxNumNodes,
        totalFundsClaimed: 0,
        numNodesClaimed: 0,
      },
      merkleRootUploadAuthority: validatorVoteAccount.publicKey,
      validatorCommissionBps: maxValidatorCommissionBps,
    };
    assertDistributionAccount(actual, expected);
  });

  it("#close_claim_status fails incorrect claimant", async () => {
    const {
      validatorVoteAccount,
      maxValidatorCommissionBps,
      tipDistributionAccount,
      validatorIdentityKeypair,
      bump,
    } = await setup_initTipDistributionAccount();
    await call_initTipDistributionAccount({
      validatorCommissionBps: maxValidatorCommissionBps,
      config: configAccount,
      validatorIdentityKeypair,
      systemProgram: SystemProgram.programId,
      merkleRootUploadAuthority: validatorVoteAccount.publicKey,
      validatorVoteAccount,
      tipDistributionAccount,
      bump,
    });

    const amount0 = 1_000_000;
    const amount1 = 2_000_000;
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        tipDistributionAccount,
        amount0 + amount1
      ),
      "confirmed"
    );
    const preBalance0 = 10000000000;
    const user0 = await generateAccount(preBalance0);
    const user1 = await generateAccount(preBalance0);
    const demoData = [
      { account: user0.publicKey, amount: new u64(amount0) },
      { account: user1.publicKey, amount: new u64(amount1) },
    ].map(({ account, amount }) => balanceToBuffer(account, amount));

    const tree = new MerkleTree(demoData);
    const root = tree.getRoot();
    const maxTotalClaim = new anchor.BN(amount0 + amount1);
    const maxNumNodes = new anchor.BN(2);

    await sleepForEpochs(1);
    await tipDistribution.methods
      .uploadMerkleRoot(root.toJSON().data, maxTotalClaim, maxNumNodes)
      .accounts({
        tipDistributionAccount,
        merkleRootUploadAuthority: validatorVoteAccount.publicKey,
        config: configAccount,
      })
      .signers([validatorVoteAccount])
      .rpc();

    const index = 0;
    const amount = new anchor.BN(amount0);
    const proof = tree.getProof(index);
    const claimant = user0;
    const [claimStatus, _bump] = await anchor.web3.PublicKey.findProgramAddress(
      [
        Buffer.from(CLAIM_STATUS_SEED, "utf8"),
        claimant.publicKey.toBuffer(),
        tipDistributionAccount.toBuffer(),
      ],
      tipDistribution.programId
    );

    await tipDistribution.methods
      .claim(_bump, amount, proof)
      .accounts({
        config: configAccount,
        tipDistributionAccount,
        claimStatus,
        claimant: claimant.publicKey,
        payer: user1.publicKey,
      })
      .signers([user1])
      .rpc();

    await sleepForEpochs(4); // wait for TDA to expire

    try {
      const acct = anchor.web3.Keypair.generate();
      await tipDistribution.methods
        .closeClaimStatus()
        .accounts({
          config: configAccount,
          claimStatus,
          claimStatusPayer: acct.publicKey, //wrong user, causes constraint check to fail
        })
        .rpc();
      assert.fail("expected exception to be thrown");
    } catch (e) {
      const err: AnchorError = e;
      assert(err.error.errorCode.code === "ConstraintRaw");
    }
  });

  it("#close_claim_status fails before TipDistributionAccount has expired with ErrorCode::PrematureCloseClaimStatus", async () => {
    const {
      validatorVoteAccount,
      maxValidatorCommissionBps,
      tipDistributionAccount,
      validatorIdentityKeypair,
      bump,
    } = await setup_initTipDistributionAccount();
    await call_initTipDistributionAccount({
      validatorCommissionBps: maxValidatorCommissionBps,
      config: configAccount,
      validatorIdentityKeypair,
      systemProgram: SystemProgram.programId,
      merkleRootUploadAuthority: validatorVoteAccount.publicKey,
      validatorVoteAccount,
      tipDistributionAccount,
      bump,
    });

    const amount0 = 1_000_000;
    const amount1 = 2_000_000;
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        tipDistributionAccount,
        amount0 + amount1
      ),
      "confirmed"
    );
    const preBalance0 = 10000000000;
    const user0 = await generateAccount(preBalance0);
    const user1 = await generateAccount(preBalance0);
    const demoData = [
      { account: user0.publicKey, amount: new u64(amount0) },
      { account: user1.publicKey, amount: new u64(amount1) },
    ].map(({ account, amount }) => balanceToBuffer(account, amount));

    const tree = new MerkleTree(demoData);
    const root = tree.getRoot();
    const maxTotalClaim = new anchor.BN(amount0 + amount1);
    const maxNumNodes = new anchor.BN(2);

    // Sleep to allow the epoch to advance
    await sleepForEpochs(1);
    await tipDistribution.methods
      .uploadMerkleRoot(root.toJSON().data, maxTotalClaim, maxNumNodes)
      .accounts({
        tipDistributionAccount,
        merkleRootUploadAuthority: validatorVoteAccount.publicKey,
        config: configAccount,
      })
      .signers([validatorVoteAccount])
      .rpc();

    const index = 0;
    const amount = new anchor.BN(amount0);
    const proof = tree.getProof(index);
    const claimant = user0;
    const [claimStatus, _bump] = await anchor.web3.PublicKey.findProgramAddress(
      [
        Buffer.from(CLAIM_STATUS_SEED, "utf8"),
        claimant.publicKey.toBuffer(),
        tipDistributionAccount.toBuffer(),
      ],
      tipDistribution.programId
    );

    await tipDistribution.methods
      .claim(_bump, amount, proof)
      .accounts({
        config: configAccount,
        tipDistributionAccount,
        claimStatus,
        claimant: claimant.publicKey,
        payer: user1.publicKey,
      })
      .signers([user1])
      .rpc();

    // should usually wait a few epochs after claiming to close the ClaimAccount
    // since we didn't wait, we cannot close the ClaimStatus account
    const balStart = await provider.connection.getBalance(user1.publicKey);
    try {
      await tipDistribution.methods
        .closeClaimStatus()
        .accounts({
          config: configAccount,
          claimStatus,
          claimStatusPayer: user1.publicKey,
        })
        .rpc();
      assert.fail("expected exception to be thrown");
    } catch (e) {
      const err: AnchorError = e;
      assert(err.error.errorCode.code === "PrematureCloseClaimStatus");
    }
    const balEnd = await provider.connection.getBalance(user1.publicKey);
    assert(balEnd === balStart);
  });

  it("#close_claim_status fails when user tries to drain TipDistributionAccount", async () => {
    const {
      validatorVoteAccount,
      maxValidatorCommissionBps,
      tipDistributionAccount,
      validatorIdentityKeypair,
      bump,
    } = await setup_initTipDistributionAccount();
    await call_initTipDistributionAccount({
      validatorCommissionBps: maxValidatorCommissionBps,
      config: configAccount,
      validatorIdentityKeypair,
      systemProgram: SystemProgram.programId,
      merkleRootUploadAuthority: validatorVoteAccount.publicKey,
      validatorVoteAccount,
      tipDistributionAccount,
      bump,
    });

    const amount0 = 1_000_000;
    const amount1 = 2_000_000;
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        tipDistributionAccount,
        amount0 + amount1
      ),
      "confirmed"
    );
    const preBalance0 = 10000000000;
    const user0 = await generateAccount(preBalance0);
    const user1 = await generateAccount(preBalance0);
    const demoData = [
      { account: user0.publicKey, amount: new u64(amount0) },
      { account: user1.publicKey, amount: new u64(amount1) },
    ].map(({ account, amount }) => balanceToBuffer(account, amount));

    const tree = new MerkleTree(demoData);
    const root = tree.getRoot();
    const maxTotalClaim = new anchor.BN(amount0 + amount1);
    const maxNumNodes = new anchor.BN(2);

    // Sleep to allow the epoch to advance
    await sleepForEpochs(1);
    await tipDistribution.methods
      .uploadMerkleRoot(root.toJSON().data, maxTotalClaim, maxNumNodes)
      .accounts({
        tipDistributionAccount,
        merkleRootUploadAuthority: validatorVoteAccount.publicKey,
        config: configAccount,
      })
      .signers([validatorVoteAccount])
      .rpc();

    const index = 0;
    const amount = new anchor.BN(amount0);
    const proof = tree.getProof(index);
    const claimant = user0;
    const [claimStatus, _bump] = await anchor.web3.PublicKey.findProgramAddress(
      [
        Buffer.from(CLAIM_STATUS_SEED, "utf8"),
        claimant.publicKey.toBuffer(),
        tipDistributionAccount.toBuffer(),
      ],
      tipDistribution.programId
    );

    await tipDistribution.methods
      .claim(_bump, amount, proof)
      .accounts({
        config: configAccount,
        tipDistributionAccount,
        claimStatus,
        claimant: claimant.publicKey,
        payer: user1.publicKey,
      })
      .signers([user1])
      .rpc();

    await sleepForEpochs(3); // wait for TDA to expire

    await tipDistribution.methods
      .closeClaimStatus()
      .accounts({
        config: configAccount,
        claimStatus,
        claimStatusPayer: user1.publicKey,
      })
      .rpc();

    try {
      // claim second time, this should fail since the TDA has expired
      await tipDistribution.methods
        .claim(_bump, amount, proof)
        .accounts({
          config: configAccount,
          tipDistributionAccount,
          claimStatus,
          claimant: claimant.publicKey,
          payer: user1.publicKey,
        })
        .signers([user1])
        .rpc();
      assert.fail("expected exception to be thrown");
    } catch (e) {
      const err: AnchorError = e;
      assert(err.error.errorCode.code === "ExpiredTipDistributionAccount");
    }
  });

  // keep this test at end, else follow test will fail with `Error: Raw transaction failed ({"err":{"InstructionError":[0,"PrivilegeEscalation"]}})`
  it("#close_claim_status happy path", async () => {
    const {
      validatorVoteAccount,
      maxValidatorCommissionBps,
      validatorIdentityKeypair,
      tipDistributionAccount,
      bump,
    } = await setup_initTipDistributionAccount();
    await call_initTipDistributionAccount({
      validatorCommissionBps: maxValidatorCommissionBps,
      config: configAccount,
      validatorIdentityKeypair,
      systemProgram: SystemProgram.programId,
      merkleRootUploadAuthority: validatorVoteAccount.publicKey,
      validatorVoteAccount,
      tipDistributionAccount,
      bump,
    });

    const amount0 = 1_000_000;
    const amount1 = 2_000_000;
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        tipDistributionAccount,
        amount0 + amount1
      ),
      "confirmed"
    );
    const preBalance0 = 10000000000;
    const user0 = await generateAccount(preBalance0);
    const user1 = await generateAccount(preBalance0);
    const demoData = [
      { account: user0.publicKey, amount: new u64(amount0) },
      { account: user1.publicKey, amount: new u64(amount1) },
    ].map(({ account, amount }) => balanceToBuffer(account, amount));

    const tree = new MerkleTree(demoData);
    const root = tree.getRoot();
    const maxTotalClaim = new anchor.BN(amount0 + amount1);
    const maxNumNodes = new anchor.BN(2);

    // Sleep to allow the epoch to advance
    await sleepForEpochs(1);
    await tipDistribution.methods
      .uploadMerkleRoot(root.toJSON().data, maxTotalClaim, maxNumNodes)
      .accounts({
        tipDistributionAccount,
        merkleRootUploadAuthority: validatorVoteAccount.publicKey,
        config: configAccount,
      })
      .signers([validatorVoteAccount])
      .rpc();

    const index = 0;
    const amount = new anchor.BN(amount0);
    const proof = tree.getProof(index);
    const claimant = user0;
    const [claimStatus, _bump] = await anchor.web3.PublicKey.findProgramAddress(
      [
        Buffer.from(CLAIM_STATUS_SEED, "utf8"),
        claimant.publicKey.toBuffer(),
        tipDistributionAccount.toBuffer(),
      ],
      tipDistribution.programId
    );

    await tipDistribution.methods
      .claim(_bump, amount, proof)
      .accounts({
        config: configAccount,
        tipDistributionAccount,
        claimStatus,
        claimant: claimant.publicKey,
        payer: user1.publicKey, //payer receives rent from closing ClaimAccount
      })
      .signers([user1])
      .rpc();

    await sleepForEpochs(4); // wait for TDA to expire

    const balStart = await provider.connection.getBalance(user1.publicKey);
    await tipDistribution.methods
      .closeClaimStatus()
      .accounts({
        config: configAccount,
        claimStatus,
        claimStatusPayer: user1.publicKey,
      })
      .rpc();

    const balEnd = await provider.connection.getBalance(user1.publicKey);
    const minRentExempt =
      await provider.connection.getMinimumBalanceForRentExemption(
        CLAIM_STATUS_LEN
      );
    assert(balEnd - balStart === minRentExempt);
  });

  it("#close_claim_status works even if TipDistributionAccount already closed", async () => {
    const {
      validatorVoteAccount,
      maxValidatorCommissionBps,
      validatorIdentityKeypair,
      tipDistributionAccount,
      bump,
    } = await setup_initTipDistributionAccount();
    await call_initTipDistributionAccount({
      validatorCommissionBps: maxValidatorCommissionBps,
      config: configAccount,
      validatorIdentityKeypair,
      systemProgram: SystemProgram.programId,
      merkleRootUploadAuthority: validatorVoteAccount.publicKey,
      validatorVoteAccount,
      tipDistributionAccount,
      bump,
    });

    const amount0 = 1_000_000;
    const amount1 = 2_000_000;
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        tipDistributionAccount,
        amount0 + amount1
      ),
      "confirmed"
    );
    const preBalance0 = 10000000000;
    const user0 = await generateAccount(preBalance0);
    const user1 = await generateAccount(preBalance0);
    const demoData = [
      { account: user0.publicKey, amount: new u64(amount0) },
      { account: user1.publicKey, amount: new u64(amount1) },
    ].map(({ account, amount }) => balanceToBuffer(account, amount));

    const tree = new MerkleTree(demoData);
    const root = tree.getRoot();
    const maxTotalClaim = new anchor.BN(amount0 + amount1);
    const maxNumNodes = new anchor.BN(2);

    // Sleep to allow the epoch to advance
    await sleepForEpochs(1);
    await tipDistribution.methods
      .uploadMerkleRoot(root.toJSON().data, maxTotalClaim, maxNumNodes)
      .accounts({
        tipDistributionAccount,
        merkleRootUploadAuthority: validatorVoteAccount.publicKey,
        config: configAccount,
      })
      .signers([validatorVoteAccount])
      .rpc();

    const index = 0;
    const amount = new anchor.BN(amount0);
    const proof = tree.getProof(index);
    const claimant = user0;
    const [claimStatus, _bump] = await anchor.web3.PublicKey.findProgramAddress(
      [
        Buffer.from(CLAIM_STATUS_SEED, "utf8"),
        claimant.publicKey.toBuffer(),
        tipDistributionAccount.toBuffer(),
      ],
      tipDistribution.programId
    );

    await tipDistribution.methods
      .claim(_bump, amount, proof)
      .accounts({
        config: configAccount,
        tipDistributionAccount,
        claimStatus,
        claimant: claimant.publicKey,
        payer: user1.publicKey,
      })
      .signers([user1])
      .rpc();

    await sleepForEpochs(3);

    const actualConfig = await tipDistribution.account.config.fetch(
      configAccount
    );
    const tda = await tipDistribution.account.tipDistributionAccount.fetch(
      tipDistributionAccount
    );

    //close the account
    await tipDistribution.methods
      .closeTipDistributionAccount(tda.epochCreatedAt)
      .accounts({
        config: configAccount,
        tipDistributionAccount,
        expiredFundsAccount: actualConfig.expiredFundsAccount,
        validatorVoteAccount: validatorVoteAccount.publicKey, //funds transferred to this account
      })
      .rpc();

    const balStart = await provider.connection.getBalance(user1.publicKey);
    await tipDistribution.methods
      .closeClaimStatus()
      .accounts({
        config: configAccount,
        claimStatus,
        claimStatusPayer: user1.publicKey,
      })
      .rpc();
    const balEnd = await provider.connection.getBalance(user1.publicKey);
    const minRentExempt =
      await provider.connection.getMinimumBalanceForRentExemption(
        CLAIM_STATUS_LEN
      );
    assert(balEnd - balStart === minRentExempt);
  });

  // move to end due to PrivilegeEscalation warning
  it("#claim happy path", async () => {
    const {
      validatorVoteAccount,
      maxValidatorCommissionBps,
      tipDistributionAccount,
      validatorIdentityKeypair,
      bump,
    } = await setup_initTipDistributionAccount();
    await call_initTipDistributionAccount({
      validatorCommissionBps: maxValidatorCommissionBps,
      config: configAccount,
      validatorIdentityKeypair,
      systemProgram: SystemProgram.programId,
      merkleRootUploadAuthority: validatorVoteAccount.publicKey,
      validatorVoteAccount,
      tipDistributionAccount,
      bump,
    });

    const amount0 = 1_000_000;
    const amount1 = 2_000_000;
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        tipDistributionAccount,
        amount0 + amount1
      ),
      "confirmed"
    );
    const preBalance0 = 10000000000;
    const user0 = await generateAccount(preBalance0);
    const user1 = await generateAccount(preBalance0);
    const demoData = [
      { account: user0.publicKey, amount: new u64(amount0) },
      { account: user1.publicKey, amount: new u64(amount1) },
    ].map(({ account, amount }) => balanceToBuffer(account, amount));

    const tree = new MerkleTree(demoData);
    const root = tree.getRoot();
    const maxTotalClaim = new anchor.BN(amount0 + amount1);
    const maxNumNodes = new anchor.BN(2);

    await sleepForEpochs(1);

    await tipDistribution.methods
      .uploadMerkleRoot(root.toJSON().data, maxTotalClaim, maxNumNodes)
      .accounts({
        tipDistributionAccount,
        merkleRootUploadAuthority: validatorVoteAccount.publicKey,
        config: configAccount,
      })
      .signers([validatorVoteAccount])
      .rpc();

    const index = 0;
    const amount = new anchor.BN(amount0);
    const proof = tree.getProof(index);
    assert(tree.verifyProof(0, proof, root));

    const claimant = user0;
    const [claimStatus, _bump] = await anchor.web3.PublicKey.findProgramAddress(
      [
        Buffer.from(CLAIM_STATUS_SEED, "utf8"),
        claimant.publicKey.toBuffer(),
        tipDistributionAccount.toBuffer(),
      ],
      tipDistribution.programId
    );

    await tipDistribution.methods
      .claim(_bump, amount, proof)
      .accounts({
        config: configAccount,
        tipDistributionAccount,
        claimStatus,
        claimant: claimant.publicKey,
        payer: user1.publicKey,
      })
      .signers([user1])
      .rpc();

    const user0Info = await tipDistribution.provider.connection.getAccountInfo(
      user0.publicKey
    );
    assert.equal(user0Info.lamports, preBalance0 + amount0);
  });
});

// utils

const sleepForEpochs = async (numEpochs: number) => {
  const [sched, epochInfo] = await Promise.all([
    provider.connection.getEpochSchedule(),
    provider.connection.getEpochInfo("confirmed"),
  ]);
  const targetEpoch = epochInfo.epoch + numEpochs;
  let currentEpoch = epochInfo.epoch;
  do {
    await sleep(sched.slotsPerEpoch * 400); //slot is usually around 400ms
    currentEpoch = (await provider.connection.getEpochInfo("confirmed")).epoch;
  } while (currentEpoch < targetEpoch);
};

const balanceToBuffer = (account: PublicKey, amount: anchor.BN): Buffer => {
  return Buffer.concat([
    account.toBuffer(),
    new u64(amount).toArrayLike(Buffer, "le", 8),
  ]);
};

const assertConfigState = (actual, expected) => {
  assert.equal(actual.authority.toString(), expected.authority.toString());
  assert.equal(
    actual.expiredFundsAccount.toString(),
    expected.expiredFundsAccount.toString()
  );
  assert.equal(
    actual.maxValidatorCommissionBps,
    expected.maxValidatorCommissionBps
  );
  assert.equal(
    actual.numEpochsValid.toString(),
    expected.numEpochsValid.toString()
  );
};

const assertDistributionAccount = (actual, expected) => {
  assert.equal(
    actual.validatorVoteAccount.toString(),
    expected.validatorVoteAccount.toString()
  );
  assert.equal(
    actual.merkleRootUploadAuthority.toString(),
    expected.merkleRootUploadAuthority.toString()
  );
  assert.equal(actual.epochCreatedAt, expected.epochCreatedAt);
  assert.equal(actual.validatorCommissionBps, expected.validatorCommissionBps);

  if (actual.merkleRoot && expected.merkleRoot) {
    assert.equal(
      actual.merkleRoot.root.toString(),
      expected.merkleRoot.root.toString()
    );
    assert.equal(
      actual.merkleRoot.maxTotalClaim.toString(),
      expected.merkleRoot.maxTotalClaim.toString()
    );
    assert.equal(
      actual.merkleRoot.maxNumNodes.toString(),
      expected.merkleRoot.maxNumNodes.toString()
    );
    assert.equal(
      actual.merkleRoot.totalFundsClaimed.toString(),
      expected.merkleRoot.totalFundsClaimed.toString()
    );
    assert.equal(
      actual.merkleRoot.numNodesClaimed.toString(),
      expected.merkleRoot.numNodesClaimed.toString()
    );
  } else if (actual.merkleRoot || expected.merkleRoot) {
    assert.fail();
  }
};

const generateAccount = async (airdropAmount: number) => {
  const account = anchor.web3.Keypair.generate();
  if (airdropAmount) {
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        account.publicKey,
        airdropAmount
      ),
      "confirmed"
    );
  }

  return account;
};

const setup_initTipDistributionAccount = async () => {
  // Fetch config state.
  const config = await tipDistribution.account.config.fetch(configAccount);

  // Create validator identity account.
  const validatorIdentityKeypair = await generateAccount(10000000000000);
  const validatorVoteAccount = anchor.web3.Keypair.generate();

  // Create validator's vote account.
  const voteInit = new VoteInit(
    validatorIdentityKeypair.publicKey,
    validatorIdentityKeypair.publicKey,
    validatorIdentityKeypair.publicKey,
    0
  );
  console.log("VoteProgram.space: ", VoteProgram.space);
  const lamports = await provider.connection.getMinimumBalanceForRentExemption(
    VoteProgram.space
  );
  const tx = VoteProgram.createAccount({
    fromPubkey: validatorIdentityKeypair.publicKey,
    votePubkey: validatorVoteAccount.publicKey,
    voteInit,
    lamports: lamports + 10 * LAMPORTS_PER_SOL,
  });
  try {
    await sendAndConfirmTransaction(provider.connection, tx, [
      validatorIdentityKeypair,
      validatorVoteAccount,
      validatorIdentityKeypair,
    ]);
  } catch (e) {
    console.log("error creating validator vote account", e);
    assert.fail(e);
  }

  // Fetch epoch info and derive TipDistributionAccount PDA.
  const epochInfo = await provider.connection.getEpochInfo("confirmed");
  const epoch = new anchor.BN(epochInfo.epoch).toArrayLike(Buffer, "le", 8);
  const [tipDistributionAccount, bump] =
    await anchor.web3.PublicKey.findProgramAddress(
      [
        Buffer.from("TIP_DISTRIBUTION_ACCOUNT", "utf8"),
        validatorVoteAccount.publicKey.toBuffer(),
        epoch,
      ],
      tipDistribution.programId
    );

  return {
    maxValidatorCommissionBps: config.maxValidatorCommissionBps,
    validatorIdentityKeypair,
    validatorVoteAccount,
    tipDistributionAccount,
    bump,
    epochInfo,
  };
};

const call_initTipDistributionAccount = async ({
  validatorCommissionBps,
  merkleRootUploadAuthority,
  config,
  systemProgram,
  // Used to sign the transaction.
  validatorIdentityKeypair,
  // The validator's vote account.
  validatorVoteAccount,
  tipDistributionAccount,
  bump,
}) => {
  return await tipDistribution.rpc.initializeTipDistributionAccount(
    merkleRootUploadAuthority,
    validatorCommissionBps,
    bump,
    {
      accounts: {
        config,
        systemProgram,
        signer: validatorIdentityKeypair.publicKey,
        validatorVoteAccount: validatorVoteAccount.publicKey,
        tipDistributionAccount,
      },
      signers: [validatorIdentityKeypair],
    }
  );
};

const sleep = (ms: number) => {
  return new Promise((resolve) => setTimeout(resolve, ms));
};
