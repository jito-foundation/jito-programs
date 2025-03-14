import { u64 } from "@saberhq/token-utils";
import * as anchor from "@coral-xyz/anchor";
import { AnchorError, Program } from "@coral-xyz/anchor";
import { assert, expect } from "chai";
import { PublicKey, VoteInit, VoteProgram } from "@solana/web3.js";
import { convertBufProofToNumber, MerkleTree } from "./merkle-tree";
import { JitoPriorityFeeDistribution } from "../target/types/jito_priority_fee_distribution";

const { SystemProgram, sendAndConfirmTransaction, LAMPORTS_PER_SOL } =
  anchor.web3;
const CONFIG_ACCOUNT_SEED = "CONFIG_ACCOUNT";
const TIP_DISTRIBUTION_ACCOUNT_LEN = 168;
const CLAIM_STATUS_SEED = "CLAIM_STATUS";
const CLAIM_STATUS_LEN = 16;
const ROOT_UPLOAD_CONFIG_SEED = "ROOT_UPLOAD_CONFIG";
const JITO_MERKLE_UPLOAD_AUTHORITY = new anchor.web3.PublicKey(
  "GZctHpWXmsZC1YHACTGGcHhYxjdRqQvTpYkb9LMvxDib"
);

const provider = anchor.AnchorProvider.local("http://127.0.0.1:8899", {
  commitment: "confirmed",
  preflightCommitment: "confirmed",
});
anchor.setProvider(provider);

const priorityFeeDistribution = anchor.workspace
  .JitoPriorityFeeDistribution as Program<JitoPriorityFeeDistribution>;

// globals
let configAccount, configBump, merkleRootUploadConfigKey;
let authority: anchor.web3.Keypair;
let expiredFundsAccount: anchor.web3.Keypair;

describe("tests priority_fee_distribution", () => {
  before(async () => {
    const [acc, bump] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from(CONFIG_ACCOUNT_SEED, "utf8")],
      priorityFeeDistribution.programId
    );
    configAccount = acc;
    configBump = bump;
    authority = await generateAccount(100 * LAMPORTS_PER_SOL);
    [merkleRootUploadConfigKey] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from(ROOT_UPLOAD_CONFIG_SEED, "utf8")],
      priorityFeeDistribution.programId
    );
  });

  it("#initialize happy path", async () => {
    // given
    const initializer = await generateAccount(100 * LAMPORTS_PER_SOL);
    expiredFundsAccount = await generateAccount(100 * LAMPORTS_PER_SOL);
    const numEpochsValid = new anchor.BN(3);
    const maxValidatorCommissionBps = 1000;

    // then
    try {
      await priorityFeeDistribution.rpc.initialize(
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
    const actualConfig = await priorityFeeDistribution.account.config.fetch(
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
    const actual =
      await priorityFeeDistribution.account.tipDistributionAccount.fetch(
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

    const actualConfig = await priorityFeeDistribution.account.config.fetch(
      configAccount
    );
    const tda =
      await priorityFeeDistribution.account.tipDistributionAccount.fetch(
        tipDistributionAccount
      );

    const balStart = await provider.connection.getBalance(
      validatorVoteAccount.publicKey
    );
    await sleepForEpochs(4);

    //close the account
    await priorityFeeDistribution.methods
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
      await priorityFeeDistribution.account.tipDistributionAccount.fetch(
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
      await priorityFeeDistribution.rpc.uploadMerkleRoot(
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

    const actual =
      await priorityFeeDistribution.account.tipDistributionAccount.fetch(
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

  it("#close_claim_status fails incorrect claimStatusPayer", async () => {
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
    await priorityFeeDistribution.methods
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
    const [claimStatus, _bump] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from(CLAIM_STATUS_SEED, "utf8"),
        claimant.publicKey.toBuffer(),
        tipDistributionAccount.toBuffer(),
      ],
      priorityFeeDistribution.programId
    );

    await priorityFeeDistribution.methods
      .claim(_bump, amount, convertBufProofToNumber(proof))
      .accounts({
        config: configAccount,
        tipDistributionAccount,
        merkleRootUploadAuthority: validatorVoteAccount.publicKey,
        claimStatus,
        claimant: claimant.publicKey,
        payer: user1.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([user1, validatorVoteAccount])
      .rpc();

    await sleepForEpochs(4); // wait for TDA to expire

    try {
      const acct = anchor.web3.Keypair.generate();
      await priorityFeeDistribution.methods
        .closeClaimStatus()
        .accounts({
          config: configAccount,
          claimStatus,
          claimStatusPayer: user1.publicKey, //wrong user, causes constraint check to fail
        })
        .rpc();
      assert.fail("expected exception to be thrown");
    } catch (e) {
      const err: AnchorError = e;
      assert.equal(err.error.errorCode.code, "ConstraintAddress");
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
    await priorityFeeDistribution.methods
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
    const [claimStatus, _bump] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from(CLAIM_STATUS_SEED, "utf8"),
        claimant.publicKey.toBuffer(),
        tipDistributionAccount.toBuffer(),
      ],
      priorityFeeDistribution.programId
    );

    await priorityFeeDistribution.methods
      .claim(_bump, amount, convertBufProofToNumber(proof))
      .accounts({
        config: configAccount,
        tipDistributionAccount,
        merkleRootUploadAuthority: validatorVoteAccount.publicKey,
        claimStatus,
        claimant: claimant.publicKey,
        payer: user1.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([user1, validatorVoteAccount])
      .rpc();

    // should usually wait a few epochs after claiming to close the ClaimAccount
    // since we didn't wait, we cannot close the ClaimStatus account
    const balStart = await provider.connection.getBalance(user1.publicKey);
    try {
      await priorityFeeDistribution.methods
        .closeClaimStatus()
        .accounts({
          config: configAccount,
          claimStatus,
          claimStatusPayer: expiredFundsAccount.publicKey,
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
    await priorityFeeDistribution.methods
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
    const [claimStatus, _bump] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from(CLAIM_STATUS_SEED, "utf8"),
        claimant.publicKey.toBuffer(),
        tipDistributionAccount.toBuffer(),
      ],
      priorityFeeDistribution.programId
    );

    await priorityFeeDistribution.methods
      .claim(_bump, amount, convertBufProofToNumber(proof))
      .accounts({
        config: configAccount,
        tipDistributionAccount,
        merkleRootUploadAuthority: validatorVoteAccount.publicKey,
        claimStatus,
        claimant: claimant.publicKey,
        payer: user1.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([user1, validatorVoteAccount])
      .rpc();

    await sleepForEpochs(3); // wait for TDA to expire

    await priorityFeeDistribution.methods
      .closeClaimStatus()
      .accounts({
        config: configAccount,
        claimStatus,
        claimStatusPayer: expiredFundsAccount.publicKey,
      })
      .rpc();

    try {
      // claim second time, this should fail since the TDA has expired
      await priorityFeeDistribution.methods
        .claim(_bump, amount, convertBufProofToNumber(proof))
        .accounts({
          config: configAccount,
          tipDistributionAccount,
          merkleRootUploadAuthority: validatorVoteAccount.publicKey,
          claimStatus,
          claimant: claimant.publicKey,
          payer: user1.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([user1, validatorVoteAccount])
        .rpc();
      assert.fail("expected exception to be thrown");
    } catch (e) {
      const err: AnchorError = e;
      assert.equal(err.error.errorCode.code, "ExpiredTipDistributionAccount");
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
    await priorityFeeDistribution.methods
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
    const [claimStatus, _bump] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from(CLAIM_STATUS_SEED, "utf8"),
        claimant.publicKey.toBuffer(),
        tipDistributionAccount.toBuffer(),
      ],
      priorityFeeDistribution.programId
    );

    await priorityFeeDistribution.methods
      .claim(_bump, amount, convertBufProofToNumber(proof))
      .accounts({
        config: configAccount,
        tipDistributionAccount,
        merkleRootUploadAuthority: validatorVoteAccount.publicKey,
        claimStatus,
        claimant: claimant.publicKey,
        payer: user1.publicKey, //payer receives rent from closing ClaimAccount
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([user1, validatorVoteAccount])
      .rpc();

    await sleepForEpochs(4); // wait for TDA to expire

    const balStart = await provider.connection.getBalance(
      expiredFundsAccount.publicKey
    );
    await priorityFeeDistribution.methods
      .closeClaimStatus()
      .accounts({
        config: configAccount,
        claimStatus,
        claimStatusPayer: expiredFundsAccount.publicKey,
      })
      .rpc();

    const balEnd = await provider.connection.getBalance(
      expiredFundsAccount.publicKey
    );
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
    await priorityFeeDistribution.methods
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
    const [claimStatus, _bump] =
      await anchor.web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from(CLAIM_STATUS_SEED, "utf8"),
          claimant.publicKey.toBuffer(),
          tipDistributionAccount.toBuffer(),
        ],
        priorityFeeDistribution.programId
      );

    await priorityFeeDistribution.methods
      .claim(_bump, amount, convertBufProofToNumber(proof))
      .accounts({
        config: configAccount,
        tipDistributionAccount,
        merkleRootUploadAuthority: validatorVoteAccount.publicKey,
        claimStatus,
        claimant: claimant.publicKey,
        payer: user1.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([user1, validatorVoteAccount])
      .rpc();

    await sleepForEpochs(3);

    const actualConfig = await priorityFeeDistribution.account.config.fetch(
      configAccount
    );
    const tda =
      await priorityFeeDistribution.account.tipDistributionAccount.fetch(
        tipDistributionAccount
      );

    //close the account
    await priorityFeeDistribution.methods
      .closeTipDistributionAccount(tda.epochCreatedAt)
      .accounts({
        config: configAccount,
        tipDistributionAccount,
        expiredFundsAccount: actualConfig.expiredFundsAccount,
        validatorVoteAccount: validatorVoteAccount.publicKey, //funds transferred to this account
      })
      .rpc();

    const balStart = await provider.connection.getBalance(
      expiredFundsAccount.publicKey
    );
    await priorityFeeDistribution.methods
      .closeClaimStatus()
      .accounts({
        config: configAccount,
        claimStatus,
        claimStatusPayer: expiredFundsAccount.publicKey,
      })
      .rpc();
    const balEnd = await provider.connection.getBalance(
      expiredFundsAccount.publicKey
    );
    const minRentExempt =
      await provider.connection.getMinimumBalanceForRentExemption(
        CLAIM_STATUS_LEN
      );
    assert(balEnd - balStart === minRentExempt);
  });

  // move to end due to PrivilegeEscalation warning
  it("#claim happy path", async () => {
    const {
      amount0,
      preBalance0,
      root,
      tipDistributionAccount,
      tree,
      user0,
      user1,
      validatorVoteAccount,
    } = await setupWithUploadedMerkleRoot();

    const index = 0;
    const amount = new anchor.BN(amount0);
    const proof = tree.getProof(index);
    assert(tree.verifyProof(0, proof, root));

    const claimant = user0;
    const [claimStatus, _bump] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from(CLAIM_STATUS_SEED, "utf8"),
        claimant.publicKey.toBuffer(),
        tipDistributionAccount.toBuffer(),
      ],
      priorityFeeDistribution.programId
    );

    await priorityFeeDistribution.methods
      .claim(_bump, amount, convertBufProofToNumber(proof))
      .accounts({
        config: configAccount,
        tipDistributionAccount,
        merkleRootUploadAuthority: validatorVoteAccount.publicKey,
        claimStatus,
        claimant: claimant.publicKey,
        payer: user1.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([user1, validatorVoteAccount])
      .rpc();

    const user0Info =
      await priorityFeeDistribution.provider.connection.getAccountInfo(
        user0.publicKey
      );
    assert.equal(user0Info.lamports, preBalance0 + amount0);
  });

  it("#claim fails if TDA merkle root upload authority not signer ", async () => {
    const { amount0, root, tipDistributionAccount, tree, user0, user1 } =
      await setupWithUploadedMerkleRoot();

    const index = 0;
    const amount = new anchor.BN(amount0);
    const proof = tree.getProof(index);
    assert(tree.verifyProof(0, proof, root));

    const claimant = user0;
    const [claimStatus, _bump] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from(CLAIM_STATUS_SEED, "utf8"),
        claimant.publicKey.toBuffer(),
        tipDistributionAccount.toBuffer(),
      ],
      priorityFeeDistribution.programId
    );

    const badAuthority = anchor.web3.Keypair.generate();

    try {
      await priorityFeeDistribution.methods
        .claim(_bump, amount, convertBufProofToNumber(proof))
        .accounts({
          config: configAccount,
          tipDistributionAccount,
          merkleRootUploadAuthority: badAuthority.publicKey,
          claimStatus,
          claimant: claimant.publicKey,
          payer: user1.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([user1, badAuthority])
        .rpc();
      assert.fail("expected exception to be thrown");
    } catch (e) {
      const err: AnchorError = e;
      assert(err.error.errorCode.code === "Unauthorized");
    }
  });

  it("#initialize_merkle_root_upload_conifg happy path", async () => {
    await setup_initTipDistributionAccount();

    const [_merkleRootUploadConfigKey, merkleRootUploadConfigBump] =
      anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from(ROOT_UPLOAD_CONFIG_SEED, "utf8")],
        priorityFeeDistribution.programId
      );
    const overrideAuthority = anchor.web3.Keypair.generate();

    const originalAuthority = anchor.web3.Keypair.generate();

    // call the init instruction
    await priorityFeeDistribution.methods
      .initializeMerkleRootUploadConfig(
        overrideAuthority.publicKey,
        originalAuthority.publicKey
      )
      .accounts({
        payer: priorityFeeDistribution.provider.publicKey,
        config: configAccount,
        authority: authority.publicKey,
        merkleRootUploadConfig: merkleRootUploadConfigKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([authority])
      .rpc({ skipPreflight: true });

    // Valdiate that the MerkleRootUploadConfig account was created
    const merkleRootUploadConfig =
      await priorityFeeDistribution.account.merkleRootUploadConfig.fetch(
        merkleRootUploadConfigKey
      );
    // Validate the MerkleRootUploadConfig authority is the Config authority
    assert.equal(merkleRootUploadConfig.bump, merkleRootUploadConfigBump);
    assert.equal(
      merkleRootUploadConfig.overrideAuthority.toString(),
      overrideAuthority.publicKey.toString()
    );
    assert.equal(
      merkleRootUploadConfig.originalUploadAuthority.toString(),
      originalAuthority.publicKey.toString()
    );
  });

  it("#update_merkle_root_upload_conifg happy path", async () => {
    await setup_initTipDistributionAccount();

    const newOverrideAuthority = anchor.web3.Keypair.generate();

    await priorityFeeDistribution.methods
      .updateMerkleRootUploadConfig(
        newOverrideAuthority.publicKey,
        JITO_MERKLE_UPLOAD_AUTHORITY
      )
      .accounts({
        config: configAccount,
        authority: authority.publicKey,
        merkleRootUploadConfig: merkleRootUploadConfigKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([authority])
      .rpc({ skipPreflight: true });

    const updatedMerkleRootUploadConfig =
      await priorityFeeDistribution.account.merkleRootUploadConfig.fetch(
        merkleRootUploadConfigKey
      );
    // Validate the MerkleRootUploadConfig authority is the new authority
    assert.equal(
      updatedMerkleRootUploadConfig.overrideAuthority.toString(),
      newOverrideAuthority.publicKey.toString()
    );
    assert.equal(
      updatedMerkleRootUploadConfig.originalUploadAuthority.toString(),
      JITO_MERKLE_UPLOAD_AUTHORITY.toString()
    );
  });

  it("#migrate_tda_merkle_root_upload_authority happy path", async () => {
    const {
      validatorVoteAccount,
      validatorIdentityKeypair,
      maxValidatorCommissionBps,
      tipDistributionAccount,
      bump,
    } = await setup_initTipDistributionAccount();
    await call_initTipDistributionAccount({
      validatorCommissionBps: maxValidatorCommissionBps,
      config: configAccount,
      validatorIdentityKeypair,
      systemProgram: SystemProgram.programId,
      merkleRootUploadAuthority: JITO_MERKLE_UPLOAD_AUTHORITY,
      validatorVoteAccount,
      tipDistributionAccount,
      bump,
    });

    const merkleRootUploadConfig =
      await priorityFeeDistribution.account.merkleRootUploadConfig.fetch(
        merkleRootUploadConfigKey
      );

    await priorityFeeDistribution.methods
      .migrateTdaMerkleRootUploadAuthority()
      .accounts({
        tipDistributionAccount: tipDistributionAccount,
        merkleRootUploadConfig: merkleRootUploadConfigKey,
      })
      .rpc({ skipPreflight: true });

    const tda =
      await priorityFeeDistribution.account.tipDistributionAccount.fetch(
        tipDistributionAccount
      );
    assert.equal(
      tda.merkleRootUploadAuthority.toString(),
      merkleRootUploadConfig.overrideAuthority.toString()
    );
  });

  it("#migrate_tda_merkle_root_upload_authority should error if TDA not Jito authority", async () => {
    const {
      validatorVoteAccount,
      validatorIdentityKeypair,
      maxValidatorCommissionBps,
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
    try {
      await priorityFeeDistribution.methods
        .migrateTdaMerkleRootUploadAuthority()
        .accounts({
          tipDistributionAccount: tipDistributionAccount,
          merkleRootUploadConfig: merkleRootUploadConfigKey,
        })
        .rpc({ skipPreflight: true });
      assert.fail("expected exception to be thrown");
    } catch (e) {
      const err: AnchorError = e;
      assert(err.error.errorCode.code === "InvalidTdaForMigration");
    }
  });

  it("#migrate_tda_merkle_root_upload_authority should error if merkle root is already uploaded", async () => {
    const {
      validatorVoteAccount,
      validatorIdentityKeypair,
      maxValidatorCommissionBps,
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
    await sleepForEpochs(1);

    await priorityFeeDistribution.methods
      .uploadMerkleRoot(root.toJSON().data, maxTotalClaim, maxNumNodes)
      .accounts({
        tipDistributionAccount,
        merkleRootUploadAuthority: validatorVoteAccount.publicKey,
        config: configAccount,
      })
      .signers([validatorVoteAccount])
      .rpc();
    try {
      await priorityFeeDistribution.methods
        .migrateTdaMerkleRootUploadAuthority()
        .accounts({
          tipDistributionAccount: tipDistributionAccount,
          merkleRootUploadConfig: merkleRootUploadConfigKey,
        })
        .rpc({ skipPreflight: true });
      assert.fail("expected exception to be thrown");
    } catch (e) {
      const err: AnchorError = e;
      assert(err.error.errorCode.code === "InvalidTdaForMigration");
    }
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
  const config = await priorityFeeDistribution.account.config.fetch(
    configAccount
  );

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
  const lamports = await provider.connection.getMinimumBalanceForRentExemption(
    VoteProgram.space
  );
  const tx = VoteProgram.createAccount({
    fromPubkey: validatorIdentityKeypair.publicKey,
    votePubkey: validatorVoteAccount.publicKey,
    voteInit,
    lamports: lamports + 10 * LAMPORTS_PER_SOL,
  });

  tx.instructions[0] = SystemProgram.createAccount({
    fromPubkey: validatorIdentityKeypair.publicKey,
    newAccountPubkey: validatorVoteAccount.publicKey,
    lamports: lamports + 10 * LAMPORTS_PER_SOL,
    space: 3762, // timely vote credits has a new vote account layout, which doesn't work correctly with solana web3.js
    programId: VoteProgram.programId,
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
    anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("TIP_DISTRIBUTION_ACCOUNT", "utf8"),
        validatorVoteAccount.publicKey.toBuffer(),
        epoch,
      ],
      priorityFeeDistribution.programId
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
  return await priorityFeeDistribution.rpc.initializeTipDistributionAccount(
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

const setupWithUploadedMerkleRoot = async () => {
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

  await priorityFeeDistribution.methods
    .uploadMerkleRoot(root.toJSON().data, maxTotalClaim, maxNumNodes)
    .accounts({
      tipDistributionAccount,
      merkleRootUploadAuthority: validatorVoteAccount.publicKey,
      config: configAccount,
    })
    .signers([validatorVoteAccount])
    .rpc();
  return {
    amount0,
    amount1,
    preBalance0,
    root,
    tipDistributionAccount,
    tree,
    user0,
    user1,
    validatorVoteAccount,
  };
};

const sleep = (ms: number) => {
  return new Promise((resolve) => setTimeout(resolve, ms));
};
