const anchor = require('@project-serum/anchor')
const assert = require('assert')
const {SystemProgram, Transaction} = anchor.web3

const CONFIG_ACCOUNT_LEN = 8 + 9 + 32 // 8 for anchor header, 9 for bumps, 32 for pubkey
const TIP_PAYMENT_ACCOUNT_LEN = 8  // 8 for header

const configAccountSeed = 'CONFIG_ACCOUNT'
const tipSeed0 = 'TIP_ACCOUNT_0'
const tipSeed1 = 'TIP_ACCOUNT_1'
const tipSeed2 = 'TIP_ACCOUNT_2'
const tipSeed3 = 'TIP_ACCOUNT_3'
const tipSeed4 = 'TIP_ACCOUNT_4'
const tipSeed5 = 'TIP_ACCOUNT_5'
const tipSeed6 = 'TIP_ACCOUNT_6'
const tipSeed7 = 'TIP_ACCOUNT_7'
let configAccount, configAccountBump, tipPaymentAccount0, tipBump0, tipPaymentAccount1, tipBump1, tipPaymentAccount2, tipBump2, tipPaymentAccount3,
    tipBump3, tipPaymentAccount4, tipBump4, tipPaymentAccount5, tipBump5, tipPaymentAccount6, tipBump6,
    tipPaymentAccount7, tipBump7

const provider = anchor.AnchorProvider.local(null, {commitment: 'confirmed', preflightCommitment: 'confirmed'},)
anchor.setProvider(provider)
const tipPaymentProg = anchor.workspace.TipPayment

describe('tests tip_payment', () => {
    const sendTip = async (accountToTip, tipAmount) => {
        const searcherKP = anchor.web3.Keypair.generate()
        const airDrop = tipAmount * 2
        await provider.connection.confirmTransaction(await provider.connection.requestAirdrop(searcherKP.publicKey, airDrop), 'confirmed',)
        const tipTx = new Transaction()
        tipTx.add(SystemProgram.transfer({
            fromPubkey: searcherKP.publicKey, toPubkey: accountToTip, lamports: tipAmount,
        }))
        await anchor.web3.sendAndConfirmTransaction(tipPaymentProg.provider.connection, tipTx, [searcherKP],)
    }
    const initializerKeys = anchor.web3.Keypair.generate()
    before(async () => {
        const [_configAccount, _configAccountBump] = await anchor.web3.PublicKey.findProgramAddress([Buffer.from(configAccountSeed, 'utf8')], tipPaymentProg.programId,)
        configAccount = _configAccount
        configAccountBump = _configAccountBump
        const [_tipPaymentAccount0, _tipBump0] = await anchor.web3.PublicKey.findProgramAddress([Buffer.from(tipSeed0, 'utf8')], tipPaymentProg.programId,)
        tipPaymentAccount0 = _tipPaymentAccount0
        tipBump0 = _tipBump0
        const [_tipPaymentAccount1, _tipBump1] = await anchor.web3.PublicKey.findProgramAddress([Buffer.from(tipSeed1, 'utf8')], tipPaymentProg.programId,)
        tipPaymentAccount1 = _tipPaymentAccount1
        tipBump1 = _tipBump1
        const [_tipPaymentAccount2, _tipBump2] = await anchor.web3.PublicKey.findProgramAddress([Buffer.from(tipSeed2, 'utf8')], tipPaymentProg.programId,)
        tipPaymentAccount2 = _tipPaymentAccount2
        tipBump2 = _tipBump2
        const [_tipPaymentAccount3, _tipBump3] = await anchor.web3.PublicKey.findProgramAddress([Buffer.from(tipSeed3, 'utf8')], tipPaymentProg.programId,)
        tipPaymentAccount3 = _tipPaymentAccount3
        tipBump3 = _tipBump3
        const [_tipPaymentAccount4, _tipBump4] = await anchor.web3.PublicKey.findProgramAddress([Buffer.from(tipSeed4, 'utf8')], tipPaymentProg.programId,)
        tipPaymentAccount4 = _tipPaymentAccount4
        tipBump4 = _tipBump4
        const [_tipPaymentAccount5, _tipBump5] = await anchor.web3.PublicKey.findProgramAddress([Buffer.from(tipSeed5, 'utf8')], tipPaymentProg.programId,)
        tipPaymentAccount5 = _tipPaymentAccount5
        tipBump5 = _tipBump5
        const [_tipPaymentAccount6, _tipBump6] = await anchor.web3.PublicKey.findProgramAddress([Buffer.from(tipSeed6, 'utf8')], tipPaymentProg.programId,)
        tipPaymentAccount6 = _tipPaymentAccount6
        tipBump6 = _tipBump6
        const [_tipPaymentAccount7, _tipBump7] = await anchor.web3.PublicKey.findProgramAddress([Buffer.from(tipSeed7, 'utf8')], tipPaymentProg.programId,)
        tipPaymentAccount7 = _tipPaymentAccount7
        tipBump7 = _tipBump7

        await provider.connection.confirmTransaction(await provider.connection.requestAirdrop(initializerKeys.publicKey, 100000000000000), 'confirmed',)
    })

    // utility function asserting all expected rent exempt accounts are indeed exempt
    const assertRentExemptAccounts = async () => {
        let minRentExempt = await tipPaymentProg.provider.connection.getMinimumBalanceForRentExemption(CONFIG_ACCOUNT_LEN)
        let accInfo = await tipPaymentProg.provider.connection.getAccountInfo(configAccount)
        assert.equal(accInfo.lamports, minRentExempt)

        minRentExempt = await tipPaymentProg.provider.connection.getMinimumBalanceForRentExemption(TIP_PAYMENT_ACCOUNT_LEN)
        accInfo = await tipPaymentProg.provider.connection.getAccountInfo(tipPaymentAccount1)
        assert.equal(accInfo.lamports, minRentExempt)

        minRentExempt = await tipPaymentProg.provider.connection.getMinimumBalanceForRentExemption(TIP_PAYMENT_ACCOUNT_LEN)
        accInfo = await tipPaymentProg.provider.connection.getAccountInfo(tipPaymentAccount2)
        assert.equal(accInfo.lamports, minRentExempt)
    }
    it('#initialize happy path', async () => {
        try {
            await tipPaymentProg.rpc.initialize({
                configAccountBump, // config
                tipBump0: tipBump0,
                tipBump1: tipBump1,
                tipBump2: tipBump2,
                tipBump3: tipBump3,
                tipBump4: tipBump4,
                tipBump5: tipBump5,
                tipBump6: tipBump6,
                tipBump7: tipBump7,
            }, {
                accounts: {
                    config: configAccount,
                    tipPaymentAccount1: tipPaymentAccount1,
                    tipPaymentAccount2: tipPaymentAccount2,
                    tipPaymentAccount3: tipPaymentAccount3,
                    tipPaymentAccount4: tipPaymentAccount4,
                    tipPaymentAccount5: tipPaymentAccount5,
                    tipPaymentAccount6: tipPaymentAccount6,
                    tipPaymentAccount7: tipPaymentAccount7,
                    tipPaymentAccount0: tipPaymentAccount0,
                    systemProgram: SystemProgram.programId,
                    payer: initializerKeys.publicKey,
                }, signers: [initializerKeys],
            },)
        } catch (e) {
            console.log('error', e)
            assert.fail()
        }
        const configState = await tipPaymentProg.account.config.fetch(configAccount)
        assert.equal(configState.tipReceiver.toString(), initializerKeys.publicKey.toString())
    })
    it('#change_tip_receiver with 0 total tips succeeds', async () => {
        let configState = await tipPaymentProg.account.config.fetch(configAccount)
        const oldTipReceiver = configState.tipReceiver
        const newTipReceiver = anchor.web3.Keypair.generate()
        await tipPaymentProg.rpc.changeTipReceiver({
            accounts: {
                config: configAccount,
                oldTipReceiver,
                newTipReceiver: newTipReceiver.publicKey,
                tipPaymentAccount1: tipPaymentAccount1,
                tipPaymentAccount2: tipPaymentAccount2,
                tipPaymentAccount3: tipPaymentAccount3,
                tipPaymentAccount4: tipPaymentAccount4,
                tipPaymentAccount5: tipPaymentAccount5,
                tipPaymentAccount6: tipPaymentAccount6,
                tipPaymentAccount7: tipPaymentAccount7,
                tipPaymentAccount0: tipPaymentAccount0,
                signer: initializerKeys.publicKey,
            }, signers: [initializerKeys],
        },)
        await assertRentExemptAccounts()
        configState = await tipPaymentProg.account.config.fetch(configAccount)
        assert.equal(configState.tipReceiver.toString(), newTipReceiver.publicKey.toString())
    })
    it('#change_tip_receiver `constraint = old_tip_receiver.key() == config.tip_receiver`', async () => {
        const badOldTipReceiver = anchor.web3.Keypair.generate().publicKey
        const newTipReceiver = anchor.web3.Keypair.generate()
        try {
            await tipPaymentProg.rpc.changeTipReceiver({
                accounts: {
                    config: configAccount,
                    oldTipReceiver: badOldTipReceiver,
                    newTipReceiver: newTipReceiver.publicKey,
                    tipPaymentAccount1: tipPaymentAccount1,
                    tipPaymentAccount2: tipPaymentAccount2,
                    tipPaymentAccount3: tipPaymentAccount3,
                    tipPaymentAccount4: tipPaymentAccount4,
                    tipPaymentAccount5: tipPaymentAccount5,
                    tipPaymentAccount6: tipPaymentAccount6,
                    tipPaymentAccount7: tipPaymentAccount7,
                    tipPaymentAccount0: tipPaymentAccount0,
                    signer: initializerKeys.publicKey,
                }, signers: [initializerKeys],
            },)
            assert.fail('expected exception to be thrown')
        } catch (e) {
            assert.equal(e.error.errorMessage, 'A raw constraint was violated')
        }
    })
    it('#claim_tips `constraint = tip_receiver.key() == config.tip_receiver`', async () => {
        const badTipReceiver = anchor.web3.Keypair.generate().publicKey
        try {
            await tipPaymentProg.rpc.claimTips({
                accounts: {
                    config: configAccount,
                    tipPaymentAccount1: tipPaymentAccount1,
                    tipPaymentAccount2: tipPaymentAccount2,
                    tipPaymentAccount3: tipPaymentAccount3,
                    tipPaymentAccount4: tipPaymentAccount4,
                    tipPaymentAccount5: tipPaymentAccount5,
                    tipPaymentAccount6: tipPaymentAccount6,
                    tipPaymentAccount7: tipPaymentAccount7,
                    tipPaymentAccount0: tipPaymentAccount0,
                    tipReceiver: badTipReceiver,
                    signer: initializerKeys.publicKey,
                }, signers: [initializerKeys],
            },)
            assert(false)
        } catch (err) {
            assert.equal(err.error.errorMessage, 'A raw constraint was violated')
        }
    })
    it('#claim_tips with bad tipPaymentAccountN', async () => {
        const configState = await tipPaymentProg.account.config.fetch(configAccount)
        const tipReceiver = configState.tipReceiver
        for (let i = 0; i < 8; i++) {
            let accounts = await getBadTipPaymentAccounts(i)
            accounts = {
                ...accounts, signer: initializerKeys.publicKey, config: configAccount, tipReceiver,
            }
            try {
                await tipPaymentProg.rpc.claimTips({
                    accounts, signers: [initializerKeys],
                },)
                assert(false)
            } catch (e) {
                assert.equal(e.error.errorMessage, 'The given account is owned by a different program than expected')
            }
        }
    })
    it('#claim_tips moves funds to correct account', async () => {
        const signer = anchor.web3.Keypair.generate()
        const tipAmount = 1000000
        await sendTip(tipPaymentAccount1, tipAmount)
        await sendTip(tipPaymentAccount2, tipAmount)
        const totalTip = tipAmount * 2

        let configState = await tipPaymentProg.account.config.fetch(configAccount)
        const tipReceiver = configState.tipReceiver
        const tipReceiverLamportsBefore = ((await tipPaymentProg.provider.connection.getAccountInfo(tipReceiver)) || {lamports: 0}).lamports
        await tipPaymentProg.rpc.claimTips({
            accounts: {
                config: configAccount,
                tipPaymentAccount1: tipPaymentAccount1,
                tipPaymentAccount2: tipPaymentAccount2,
                tipPaymentAccount3: tipPaymentAccount3,
                tipPaymentAccount4: tipPaymentAccount4,
                tipPaymentAccount5: tipPaymentAccount5,
                tipPaymentAccount6: tipPaymentAccount6,
                tipPaymentAccount7: tipPaymentAccount7,
                tipPaymentAccount0: tipPaymentAccount0,
                tipReceiver: tipReceiver,
                signer: signer.publicKey,
            }, signers: [signer],
        },)

        await assertRentExemptAccounts()
        const tipReceiverLamportsAfter = (await tipPaymentProg.provider.connection.getAccountInfo(tipReceiver)).lamports
        assert.equal(tipReceiverLamportsAfter - tipReceiverLamportsBefore, totalTip)
    })
    it('#set_tip_receiver transfers funds to previous tip_receiver', async () => {
        const tipAmount = 10000000
        await sendTip(tipPaymentAccount1, tipAmount)
        await sendTip(tipPaymentAccount2, tipAmount)
        const totalTip = tipAmount * 2

        let configState = await tipPaymentProg.account.config.fetch(configAccount)
        const oldTipReceiver = configState.tipReceiver
        const oldTipReceiverBalanceBefore = (await tipPaymentProg.provider.connection.getAccountInfo(oldTipReceiver)).lamports
        const newTipReceiver = anchor.web3.Keypair.generate()
        const newLeader = anchor.web3.Keypair.generate()
        await tipPaymentProg.rpc.changeTipReceiver({
            accounts: {
                oldTipReceiver,
                newTipReceiver: newTipReceiver.publicKey,
                config: configAccount,
                signer: newLeader.publicKey,
                tipPaymentAccount1: tipPaymentAccount1,
                tipPaymentAccount2: tipPaymentAccount2,
                tipPaymentAccount3: tipPaymentAccount3,
                tipPaymentAccount4: tipPaymentAccount4,
                tipPaymentAccount5: tipPaymentAccount5,
                tipPaymentAccount6: tipPaymentAccount6,
                tipPaymentAccount7: tipPaymentAccount7,
                tipPaymentAccount0: tipPaymentAccount0,
            }, signers: [newLeader],
        },)
        await assertRentExemptAccounts()
        const oldTipReceiverBalanceAfter = (await tipPaymentProg.provider.connection.getAccountInfo(oldTipReceiver)).lamports
        assert.equal(oldTipReceiverBalanceAfter, totalTip + oldTipReceiverBalanceBefore)
    })
})


// utils

const getBadTipPaymentAccounts = async (n) => {
    const badTipPaymentAccount = anchor.web3.Keypair.generate().publicKey
    await provider.connection.confirmTransaction(await provider.connection.requestAirdrop(badTipPaymentAccount, 100000000000), 'confirmed',)
    let accs = {
        tipPaymentAccount0: tipPaymentAccount0,
        tipPaymentAccount1: tipPaymentAccount1,
        tipPaymentAccount2: tipPaymentAccount2,
        tipPaymentAccount3: tipPaymentAccount3,
        tipPaymentAccount4: tipPaymentAccount4,
        tipPaymentAccount5: tipPaymentAccount5,
        tipPaymentAccount6: tipPaymentAccount6,
        tipPaymentAccount7: tipPaymentAccount7,
    }
    switch (n) {
        case 0:
            return {
                ...accs, tipPaymentAccount0: badTipPaymentAccount,
            }
        case 1:
            return {
                ...accs, tipPaymentAccount1: badTipPaymentAccount,
            }
        case 2:
            return {
                ...accs, tipPaymentAccount2: badTipPaymentAccount,
            }
        case 3:
            return {
                ...accs, tipPaymentAccount3: badTipPaymentAccount,
            }
        case 4:
            return {
                ...accs, tipPaymentAccount4: badTipPaymentAccount,
            }
        case 5:
            return {
                ...accs, tipPaymentAccount5: badTipPaymentAccount,
            }
        case 6:
            return {
                ...accs, tipPaymentAccount6: badTipPaymentAccount,
            }
        case 7:
            return {
                ...accs, tipPaymentAccount7: badTipPaymentAccount,
            }
        default:
            return undefined
    }
}
