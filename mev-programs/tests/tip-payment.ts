import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";

import { TipPayment } from "../target/types/tip_payment";
import { assert } from "chai";
import { PublicKey } from "@solana/web3.js";

const { SystemProgram, Transaction } = anchor.web3;

const configAccountSeed = "CONFIG_ACCOUNT";
const tipSeed0 = "TIP_ACCOUNT_0";
const tipSeed1 = "TIP_ACCOUNT_1";
const tipSeed2 = "TIP_ACCOUNT_2";
const tipSeed3 = "TIP_ACCOUNT_3";
const tipSeed4 = "TIP_ACCOUNT_4";
const tipSeed5 = "TIP_ACCOUNT_5";
const tipSeed6 = "TIP_ACCOUNT_6";
const tipSeed7 = "TIP_ACCOUNT_7";
let configAccount,
  configAccountBump,
  tipPaymentAccount0,
  tipBump0,
  tipPaymentAccount1,
  tipBump1,
  tipPaymentAccount2,
  tipBump2,
  tipPaymentAccount3,
  tipBump3,
  tipPaymentAccount4,
  tipBump4,
  tipPaymentAccount5,
  tipBump5,
  tipPaymentAccount6,
  tipBump6,
  tipPaymentAccount7,
  tipBump7,
  tipAccounts;

const provider = anchor.AnchorProvider.local("http://127.0.0.1:8899", {
  commitment: "confirmed",
  preflightCommitment: "confirmed",
});
anchor.setProvider(provider);
const tipPaymentProg = anchor.workspace.TipPayment as Program<TipPayment>;

describe("tests tip_payment", () => {
  const sendTip = async (accountToTip: PublicKey, tipAmount: number) => {
    const searcherKP = anchor.web3.Keypair.generate();
    const airDrop = tipAmount * 2;
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(searcherKP.publicKey, airDrop),
      "confirmed"
    );
    const tipTx = new Transaction();
    tipTx.add(
      SystemProgram.transfer({
        fromPubkey: searcherKP.publicKey,
        toPubkey: accountToTip,
        lamports: tipAmount,
      })
    );
    await anchor.web3.sendAndConfirmTransaction(
      tipPaymentProg.provider.connection,
      tipTx,
      [searcherKP]
    );
  };
  const initializerKeys = anchor.web3.Keypair.generate();
  const blockProducerKeys = anchor.web3.Keypair.generate();
  before(async () => {
    const [_configAccount, _configAccountBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from(configAccountSeed, "utf8")],
        tipPaymentProg.programId
      );
    configAccount = _configAccount;
    configAccountBump = _configAccountBump;
    const [_tipPaymentAccount0, _tipBump0] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from(tipSeed0, "utf8")],
        tipPaymentProg.programId
      );
    tipPaymentAccount0 = _tipPaymentAccount0;
    tipBump0 = _tipBump0;
    const [_tipPaymentAccount1, _tipBump1] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from(tipSeed1, "utf8")],
        tipPaymentProg.programId
      );
    tipPaymentAccount1 = _tipPaymentAccount1;
    tipBump1 = _tipBump1;
    const [_tipPaymentAccount2, _tipBump2] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from(tipSeed2, "utf8")],
        tipPaymentProg.programId
      );
    tipPaymentAccount2 = _tipPaymentAccount2;
    tipBump2 = _tipBump2;
    const [_tipPaymentAccount3, _tipBump3] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from(tipSeed3, "utf8")],
        tipPaymentProg.programId
      );
    tipPaymentAccount3 = _tipPaymentAccount3;
    tipBump3 = _tipBump3;
    const [_tipPaymentAccount4, _tipBump4] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from(tipSeed4, "utf8")],
        tipPaymentProg.programId
      );
    tipPaymentAccount4 = _tipPaymentAccount4;
    tipBump4 = _tipBump4;
    const [_tipPaymentAccount5, _tipBump5] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from(tipSeed5, "utf8")],
        tipPaymentProg.programId
      );
    tipPaymentAccount5 = _tipPaymentAccount5;
    tipBump5 = _tipBump5;
    const [_tipPaymentAccount6, _tipBump6] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from(tipSeed6, "utf8")],
        tipPaymentProg.programId
      );
    tipPaymentAccount6 = _tipPaymentAccount6;
    tipBump6 = _tipBump6;
    const [_tipPaymentAccount7, _tipBump7] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from(tipSeed7, "utf8")],
        tipPaymentProg.programId
      );
    tipPaymentAccount7 = _tipPaymentAccount7;
    tipBump7 = _tipBump7;

    tipAccounts = {
      tipPaymentAccount0: tipPaymentAccount0,
      tipPaymentAccount1: tipPaymentAccount1,
      tipPaymentAccount2: tipPaymentAccount2,
      tipPaymentAccount3: tipPaymentAccount3,
      tipPaymentAccount4: tipPaymentAccount4,
      tipPaymentAccount5: tipPaymentAccount5,
      tipPaymentAccount6: tipPaymentAccount6,
      tipPaymentAccount7: tipPaymentAccount7,
    };

    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        initializerKeys.publicKey,
        100000000000000
      ),
      "confirmed"
    );
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        blockProducerKeys.publicKey,
        100000000000000
      ),
      "confirmed"
    );
  });

  // utility function asserting all expected rent exempt accounts are indeed exempt
  const assertRentExemptAccounts = async (
    tip_payment_account_pubkey: PublicKey
  ) => {
    const tip_payment_account =
      await tipPaymentProg.provider.connection.getAccountInfo(
        tip_payment_account_pubkey
      );
    const minRentExempt =
      await tipPaymentProg.provider.connection.getMinimumBalanceForRentExemption(
        tip_payment_account.data.length
      );
    assert.equal(tip_payment_account.lamports, minRentExempt);
  };

  it("#initialize happy path", async () => {
    try {
      await tipPaymentProg.rpc.initialize(
        {
          config: configAccountBump,
          tipPaymentAccount0: tipBump0,
          tipPaymentAccount1: tipBump1,
          tipPaymentAccount2: tipBump2,
          tipPaymentAccount3: tipBump3,
          tipPaymentAccount4: tipBump4,
          tipPaymentAccount5: tipBump5,
          tipPaymentAccount6: tipBump6,
          tipPaymentAccount7: tipBump7,
        },
        {
          accounts: {
            config: configAccount,
            systemProgram: SystemProgram.programId,
            payer: initializerKeys.publicKey,
            ...tipAccounts,
          },
          signers: [initializerKeys],
        }
      );
    } catch (e) {
      assert.fail();
    }
    const configState = await tipPaymentProg.account.config.fetch(
      configAccount
    );
    assert.equal(
      configState.tipReceiver.toString(),
      initializerKeys.publicKey.toString()
    );
  });
  it("#change_tip_receiver with 0 total tips succeeds", async () => {
    let configState = await tipPaymentProg.account.config.fetch(configAccount);
    const oldTipReceiver = configState.tipReceiver;
    const blockBuilder = configState.blockBuilder;
    const newTipReceiver = anchor.web3.Keypair.generate();
    await tipPaymentProg.rpc.changeTipReceiver({
      accounts: {
        config: configAccount,
        oldTipReceiver,
        newTipReceiver: newTipReceiver.publicKey,
        blockBuilder,
        signer: initializerKeys.publicKey,
        ...tipAccounts,
      },
      signers: [initializerKeys],
    });
    await assertRentExemptAccounts(tipPaymentAccount0);
    configState = await tipPaymentProg.account.config.fetch(configAccount);
    assert.equal(
      configState.tipReceiver.toString(),
      newTipReceiver.publicKey.toString()
    );
  });
  it("#change_tip_receiver `constraint = old_tip_receiver.key() == config.tip_receiver`", async () => {
    const badOldTipReceiver = anchor.web3.Keypair.generate().publicKey;
    const newTipReceiver = anchor.web3.Keypair.generate();

    let configState = await tipPaymentProg.account.config.fetch(configAccount);
    const blockBuilder = configState.blockBuilder;

    try {
      await tipPaymentProg.rpc.changeTipReceiver({
        accounts: {
          config: configAccount,
          oldTipReceiver: badOldTipReceiver,
          newTipReceiver: newTipReceiver.publicKey,
          blockBuilder,
          signer: initializerKeys.publicKey,
          ...tipAccounts,
        },
        signers: [initializerKeys],
      });
      assert.fail("expected exception to be thrown");
    } catch (e) {
      assert.equal(e.error.errorMessage, "A raw constraint was violated");
      assert.equal(e.error.origin, "old_tip_receiver");
    }
  });
  it("#change_tip_receiver `constraint = block_builder.key() == config.block_builder`", async () => {
    let configState = await tipPaymentProg.account.config.fetch(configAccount);
    const oldTipReceiver = configState.tipReceiver;

    const badBlockBuilder = anchor.web3.Keypair.generate().publicKey;

    try {
      await tipPaymentProg.rpc.changeTipReceiver({
        accounts: {
          config: configAccount,
          oldTipReceiver: oldTipReceiver,
          newTipReceiver: oldTipReceiver,
          blockBuilder: badBlockBuilder,
          signer: initializerKeys.publicKey,
          ...tipAccounts,
        },
        signers: [initializerKeys],
      });
      assert.fail("expected exception to be thrown");
    } catch (e) {
      assert.equal(e.error.errorMessage, "A raw constraint was violated");
      assert.equal(e.error.origin, "block_builder");
    }
  });
  it("#claim_tips `constraint = tip_receiver.key() == config.tip_receiver`", async () => {
    const badTipReceiver = anchor.web3.Keypair.generate().publicKey;

    let configState = await tipPaymentProg.account.config.fetch(configAccount);
    const blockBuilder = configState.blockBuilder;

    try {
      await tipPaymentProg.rpc.claimTips({
        accounts: {
          config: configAccount,
          tipReceiver: badTipReceiver,
          blockBuilder,
          signer: initializerKeys.publicKey,
          ...tipAccounts,
        },
        signers: [initializerKeys],
      });
      assert(false);
    } catch (e) {
      assert.equal(e.error.errorMessage, "A raw constraint was violated");
      assert.equal(e.error.origin, "tip_receiver");
    }
  });
  it("#claim_tips `constraint = config.block_builder == block_builder.key()`", async () => {
    let configState = await tipPaymentProg.account.config.fetch(configAccount);
    const tipReceiver = configState.tipReceiver;
    const badBlockBuilder = anchor.web3.Keypair.generate().publicKey;

    try {
      await tipPaymentProg.rpc.claimTips({
        accounts: {
          config: configAccount,
          tipReceiver: tipReceiver,
          blockBuilder: badBlockBuilder,
          signer: initializerKeys.publicKey,
          ...tipAccounts,
        },
        signers: [initializerKeys],
      });
      assert(false);
    } catch (e) {
      assert.equal(e.error.errorMessage, "A raw constraint was violated");
      assert.equal(e.error.origin, "block_builder");
    }
  });
  it("#change_block_builder constraint = old_tip_receiver.key() == config.tip_receiver", async () => {
    const badTipReceiver = anchor.web3.Keypair.generate().publicKey;

    let configState = await tipPaymentProg.account.config.fetch(configAccount);
    const oldBlockBuilder = configState.blockBuilder;

    try {
      await tipPaymentProg.rpc.changeBlockBuilder(new anchor.BN(0), {
        accounts: {
          config: configAccount,
          tipReceiver: badTipReceiver,
          oldBlockBuilder: oldBlockBuilder,
          newBlockBuilder: oldBlockBuilder,
          signer: initializerKeys.publicKey,
          ...tipAccounts,
        },
        signers: [initializerKeys],
      });
      assert(false);
    } catch (e) {
      assert.equal(e.error.errorMessage, "A raw constraint was violated");
      assert.equal(e.error.origin, "tip_receiver");
    }
  });
  it("#change_block_builder constraint = old_block_builder.key() == config.block_builder", async () => {
    const badBlockBuilder = anchor.web3.Keypair.generate().publicKey;

    let configState = await tipPaymentProg.account.config.fetch(configAccount);
    const tipReceiver = configState.tipReceiver;

    try {
      await tipPaymentProg.rpc.changeBlockBuilder(new anchor.BN(0), {
        accounts: {
          config: configAccount,
          tipReceiver,
          oldBlockBuilder: badBlockBuilder,
          newBlockBuilder: badBlockBuilder,
          signer: initializerKeys.publicKey,
          ...tipAccounts,
        },
        signers: [initializerKeys],
      });
      assert(false);
    } catch (e) {
      assert.equal(e.error.errorMessage, "A raw constraint was violated");
      assert.equal(e.error.origin, "old_block_builder");
    }
  });
  it("#change_block_builder denominator greater than 100", async () => {
    let configState = await tipPaymentProg.account.config.fetch(configAccount);
    const tipReceiver = configState.tipReceiver;
    const blockBuilder = configState.blockBuilder;
    try {
      await tipPaymentProg.rpc.changeBlockBuilder(new anchor.BN(101), {
        accounts: {
          config: configAccount,
          tipReceiver,
          oldBlockBuilder: blockBuilder,
          newBlockBuilder: blockBuilder,
          signer: initializerKeys.publicKey,
          ...tipAccounts,
        },
        signers: [initializerKeys],
      });
      assert(false);
    } catch (e) {
      assert.equal(e.error.errorMessage, "InvalidFee");
    }
  });
  it("#claim_tips with bad tipPaymentAccountN", async () => {
    const configState = await tipPaymentProg.account.config.fetch(
      configAccount
    );

    const tipReceiver = configState.tipReceiver;
    const blockBuilder = configState.blockBuilder;

    for (let i = 0; i < 8; i++) {
      let accounts = await getBadTipPaymentAccounts(i);
      accounts = {
        ...accounts,
        signer: initializerKeys.publicKey,
        config: configAccount,
        tipReceiver,
        blockBuilder,
      };
      try {
        await tipPaymentProg.rpc.claimTips({
          accounts,
          signers: [initializerKeys],
        });
        assert(false);
      } catch (e) {
        assert.equal(
          e.error.errorMessage,
          "The given account is owned by a different program than expected"
        );
      }
    }
  });
  it("#claim_tips moves funds to the tip receiver and block builder", async () => {
    let configState = await tipPaymentProg.account.config.fetch(configAccount);
    const tipReceiver = configState.tipReceiver;
    const blockBuilder = configState.blockBuilder;

    // Set block builder to take 50% cut
    await tipPaymentProg.rpc.changeBlockBuilder(new anchor.BN(50), {
      accounts: {
        config: configAccount,
        tipReceiver,
        oldBlockBuilder: blockBuilder,
        newBlockBuilder: blockBuilder,
        signer: initializerKeys.publicKey,
        ...tipAccounts,
      },
      signers: [initializerKeys],
    });

    const tipAmount = 100000000;
    await sendTip(tipPaymentAccount1, tipAmount);
    await sendTip(tipPaymentAccount2, tipAmount);
    const totalTips = tipAmount * 2;

    const tipReceiverLamportsBefore = (
      (await tipPaymentProg.provider.connection.getAccountInfo(
        tipReceiver
      )) ?? { lamports: 0 }
    ).lamports;
    const blockBuilderLamportsBefore = (
      (await tipPaymentProg.provider.connection.getAccountInfo(
        blockBuilder
      )) ?? { lamports: 0 }
    ).lamports;

    await tipPaymentProg.rpc.claimTips({
      accounts: {
        config: configAccount,
        tipReceiver: tipReceiver,
        blockBuilder,
        signer: initializerKeys.publicKey,
        ...tipAccounts,
      },
      signers: [initializerKeys],
    });

    await assertRentExemptAccounts(tipPaymentAccount0);
    const tipReceiverLamportsAfter = (
      await tipPaymentProg.provider.connection.getAccountInfo(tipReceiver)
    ).lamports;
    const blockBuilderLamportsAfter = (
      await tipPaymentProg.provider.connection.getAccountInfo(blockBuilder)
    ).lamports;
    assert.equal(
      tipReceiverLamportsAfter - tipReceiverLamportsBefore,
      totalTips / 2
    );
    assert.equal(
      blockBuilderLamportsAfter - blockBuilderLamportsBefore,
      totalTips / 2
    );
  });
  it("#set_tip_receiver transfers funds to previous tip_receiver and block builder", async () => {
    const tipAmount = 10000000;
    await sendTip(tipPaymentAccount1, tipAmount);
    await sendTip(tipPaymentAccount2, tipAmount);
    const totalTip = tipAmount * 2;

    let configState = await tipPaymentProg.account.config.fetch(configAccount);
    const oldTipReceiver = configState.tipReceiver;

    const blockBuilder = configState.blockBuilder;
    const tipReceiverLamportsBefore = (
      (await tipPaymentProg.provider.connection.getAccountInfo(
        oldTipReceiver
      )) ?? { lamports: 0 }
    ).lamports;
    const blockBuilderLamportsBefore = (
      (await tipPaymentProg.provider.connection.getAccountInfo(
        blockBuilder
      )) ?? { lamports: 0 }
    ).lamports;

    const newTipReceiver = anchor.web3.Keypair.generate();
    await tipPaymentProg.rpc.changeTipReceiver({
      accounts: {
        oldTipReceiver,
        newTipReceiver: newTipReceiver.publicKey,
        config: configAccount,
        blockBuilder,
        signer: initializerKeys.publicKey,
        ...tipAccounts,
      },
      signers: [initializerKeys],
    });
    await assertRentExemptAccounts(tipPaymentAccount0);
    const oldTipReceiverBalanceAfter = (
      await tipPaymentProg.provider.connection.getAccountInfo(oldTipReceiver)
    ).lamports;
    const blockBuilderBalanceAfter = (
      await tipPaymentProg.provider.connection.getAccountInfo(blockBuilder)
    ).lamports;
    assert.equal(
      oldTipReceiverBalanceAfter - tipReceiverLamportsBefore,
      totalTip / 2
    );
    assert.equal(
      blockBuilderBalanceAfter - blockBuilderLamportsBefore,
      totalTip / 2
    );

    configState = await tipPaymentProg.account.config.fetch(configAccount);
    assert.equal(
      configState.tipReceiver.toString(),
      newTipReceiver.publicKey.toString()
    );
  });

  it("#change_block_builder transfers funds to previous tip_receiver and block builder", async () => {
    const tipAmount = 10000000;
    await sendTip(tipPaymentAccount1, tipAmount);
    await sendTip(tipPaymentAccount2, tipAmount);
    const totalTip = tipAmount * 2;

    let configState = await tipPaymentProg.account.config.fetch(configAccount);
    const tipReceiver = configState.tipReceiver;

    const blockBuilder = configState.blockBuilder;
    const tipReceiverLamportsBefore = (
      (await tipPaymentProg.provider.connection.getAccountInfo(
        tipReceiver
      )) ?? { lamports: 0 }
    ).lamports;
    const blockBuilderLamportsBefore = (
      (await tipPaymentProg.provider.connection.getAccountInfo(
        blockBuilder
      )) ?? { lamports: 0 }
    ).lamports;

    const newBlockBuilder = anchor.web3.Keypair.generate();
    await tipPaymentProg.rpc.changeBlockBuilder(new anchor.BN(75), {
      accounts: {
        config: configAccount,
        tipReceiver,
        oldBlockBuilder: blockBuilder,
        newBlockBuilder: newBlockBuilder.publicKey,
        signer: initializerKeys.publicKey,
        ...tipAccounts,
      },
      signers: [initializerKeys],
    });
    await assertRentExemptAccounts(tipPaymentAccount0);
    const oldTipReceiverBalanceAfter = (
      await tipPaymentProg.provider.connection.getAccountInfo(tipReceiver)
    ).lamports;
    const blockBuilderBalanceAfter = (
      await tipPaymentProg.provider.connection.getAccountInfo(blockBuilder)
    ).lamports;
    assert.equal(
      oldTipReceiverBalanceAfter - tipReceiverLamportsBefore,
      totalTip / 2
    );
    assert.equal(
      blockBuilderBalanceAfter - blockBuilderLamportsBefore,
      totalTip / 2
    );

    configState = await tipPaymentProg.account.config.fetch(configAccount);
    assert.equal(
      configState.blockBuilder.toString(),
      newBlockBuilder.publicKey.toString()
    );
    assert.equal(
      configState.blockBuilderCommissionPct.toString(),
      new anchor.BN(75).toString()
    );
  });
});

// utils

const getBadTipPaymentAccounts = async (n: number) => {
  const badTipPaymentAccount = anchor.web3.Keypair.generate().publicKey;
  await provider.connection.confirmTransaction(
    await provider.connection.requestAirdrop(
      badTipPaymentAccount,
      100000000000
    ),
    "confirmed"
  );
  switch (n) {
    case 0:
      return {
        ...tipAccounts,
        tipPaymentAccount0: badTipPaymentAccount,
      };
    case 1:
      return {
        ...tipAccounts,
        tipPaymentAccount1: badTipPaymentAccount,
      };
    case 2:
      return {
        ...tipAccounts,
        tipPaymentAccount2: badTipPaymentAccount,
      };
    case 3:
      return {
        ...tipAccounts,
        tipPaymentAccount3: badTipPaymentAccount,
      };
    case 4:
      return {
        ...tipAccounts,
        tipPaymentAccount4: badTipPaymentAccount,
      };
    case 5:
      return {
        ...tipAccounts,
        tipPaymentAccount5: badTipPaymentAccount,
      };
    case 6:
      return {
        ...tipAccounts,
        tipPaymentAccount6: badTipPaymentAccount,
      };
    case 7:
      return {
        ...tipAccounts,
        tipPaymentAccount7: badTipPaymentAccount,
      };
    default:
      return undefined;
  }
};
