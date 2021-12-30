const anchor = require( '@project-serum/anchor' )
const assert = require( 'assert' )
const { SystemProgram, Transaction } = anchor.web3

const CONFIG_ACCOUNT_LEN = 8 + 40
const MEV_PAYMENT_ACCOUNT_LEN = 8

const configAccountSeed = 'CONFIG_ACCOUNT'
const mevSeed1 = 'MEV_ACCOUNT_1'
const mevSeed2 = 'MEV_ACCOUNT_2'
const mevSeed3 = 'MEV_ACCOUNT_3'
const mevSeed4 = 'MEV_ACCOUNT_4'
const mevSeed5 = 'MEV_ACCOUNT_5'
const mevSeed6 = 'MEV_ACCOUNT_6'
const mevSeed7 = 'MEV_ACCOUNT_7'
const mevSeed8 = 'MEV_ACCOUNT_8'

describe( 'tests payment_vault', () => {
    const sendTip = async ( accountToTip, tipAmount ) => {
        const searcherKP = anchor.web3.Keypair.generate()
        const airDrop = tipAmount * 2
        await provider.connection.confirmTransaction(
            await provider.connection.requestAirdrop(
                searcherKP.publicKey, airDrop
            ),
            'confirmed',
        )
        const tipTx = new Transaction()
        tipTx.add(
            SystemProgram.transfer({
                fromPubkey: searcherKP.publicKey,
                toPubkey: accountToTip,
                lamports: tipAmount,
            })
        )
        await anchor.web3.sendAndConfirmTransaction(
            paymentVaultProg.provider.connection,
            tipTx,
            [ searcherKP ],
        )
    }
    const provider = anchor.Provider.local(
      undefined,
      { commitment: 'confirmed', preflightCommitment: 'confirmed' },
    )
    anchor.setProvider( provider )
    const paymentVaultProg = anchor.workspace.PaymentVault
    const initializerKeys = anchor.web3.Keypair.generate()
    let configAccount, configAccountBump,
        mevPaymentAccount1, mevBump1,
        mevPaymentAccount2, mevBump2,
        mevPaymentAccount3, mevBump3,
        mevPaymentAccount4, mevBump4,
        mevPaymentAccount5, mevBump5,
        mevPaymentAccount6, mevBump6,
        mevPaymentAccount7, mevBump7,
        mevPaymentAccount8, mevBump8
    before( async () => {
        const [_configAccount, _configAccountBump] = await anchor.web3.PublicKey.findProgramAddress(
            [Buffer.from( configAccountSeed, 'utf8' )],
            paymentVaultProg.programId,
        )
        configAccount = _configAccount
        configAccountBump = _configAccountBump
        const [_mevPaymentAccount1, _mevBump1] = await anchor.web3.PublicKey.findProgramAddress(
            [Buffer.from( mevSeed1, 'utf8' )],
            paymentVaultProg.programId,
        )
        mevPaymentAccount1 = _mevPaymentAccount1
        mevBump1 = _mevBump1
        const [_mevPaymentAccount2, _mevBump2] = await anchor.web3.PublicKey.findProgramAddress(
            [Buffer.from( mevSeed2, 'utf8' )],
            paymentVaultProg.programId,
        )
        mevPaymentAccount2 = _mevPaymentAccount2
        mevBump2 = _mevBump2
        const [_mevPaymentAccount3, _mevBump3] = await anchor.web3.PublicKey.findProgramAddress(
            [Buffer.from( mevSeed3, 'utf8' )],
            paymentVaultProg.programId,
        )
        mevPaymentAccount3 = _mevPaymentAccount3
        mevBump3 = _mevBump3
        const [_mevPaymentAccount4, _mevBump4] = await anchor.web3.PublicKey.findProgramAddress(
            [Buffer.from( mevSeed4, 'utf8' )],
            paymentVaultProg.programId,
        )
        mevPaymentAccount4 = _mevPaymentAccount4
        mevBump4 = _mevBump4
        const [_mevPaymentAccount5, _mevBump5] = await anchor.web3.PublicKey.findProgramAddress(
            [Buffer.from( mevSeed5, 'utf8' )],
            paymentVaultProg.programId,
        )
        mevPaymentAccount5 = _mevPaymentAccount5
        mevBump5 = _mevBump5
        const [_mevPaymentAccount6, _mevBump6] = await anchor.web3.PublicKey.findProgramAddress(
            [Buffer.from( mevSeed6, 'utf8' )],
            paymentVaultProg.programId,
        )
        mevPaymentAccount6 = _mevPaymentAccount6
        mevBump6 = _mevBump6
        const [_mevPaymentAccount7, _mevBump7] = await anchor.web3.PublicKey.findProgramAddress(
            [Buffer.from( mevSeed7, 'utf8' )],
            paymentVaultProg.programId,
        )
        mevPaymentAccount7 = _mevPaymentAccount7
        mevBump7 = _mevBump7
        const [_mevPaymentAccount8, _mevBump8] = await anchor.web3.PublicKey.findProgramAddress(
            [Buffer.from( mevSeed8, 'utf8' )],
            paymentVaultProg.programId,
        )
        mevPaymentAccount8 = _mevPaymentAccount8
        mevBump8 = _mevBump8

        await provider.connection.confirmTransaction(
            await provider.connection.requestAirdrop(
                initializerKeys.publicKey, 100000000000000
            ),
            'confirmed',
        )
    })

    // utility function asserting all expected rent exempt accounts are indeed exempt
    const assertRentExemptAccounts = async () => {
        let minRentExempt = await paymentVaultProg.provider.connection.getMinimumBalanceForRentExemption( CONFIG_ACCOUNT_LEN )
        let accInfo = await paymentVaultProg.provider.connection.getAccountInfo( configAccount )
        assert.equal( accInfo.lamports, minRentExempt )

        minRentExempt = await paymentVaultProg.provider.connection.getMinimumBalanceForRentExemption( MEV_PAYMENT_ACCOUNT_LEN )
        accInfo = await paymentVaultProg.provider.connection.getAccountInfo( mevPaymentAccount1 )
        assert.equal( accInfo.lamports, minRentExempt )

        minRentExempt = await paymentVaultProg.provider.connection.getMinimumBalanceForRentExemption( MEV_PAYMENT_ACCOUNT_LEN )
        accInfo = await paymentVaultProg.provider.connection.getAccountInfo( mevPaymentAccount2 )
        assert.equal( accInfo.lamports, minRentExempt )
    }
    it( '#initialize happy path', async () => {
        await paymentVaultProg.rpc.initialize(
            {
                configAccountBump,
                mevBump1,
                mevBump2,
                mevBump3,
                mevBump4,
                mevBump5,
                mevBump6,
                mevBump7,
                mevBump8,
            },
            {
              accounts: {
                  config: configAccount,
                  initialTipClaimer: initializerKeys.publicKey,
                  payer: initializerKeys.publicKey,
                  systemProgram: SystemProgram.programId,
                  mevPaymentAccount1,
                  mevPaymentAccount2,
                  mevPaymentAccount3,
                  mevPaymentAccount4,
                  mevPaymentAccount5,
                  mevPaymentAccount6,
                  mevPaymentAccount7,
                  mevPaymentAccount8,
              },
              signers: [initializerKeys],
            },
        )
        const configState = await paymentVaultProg.account.config.fetch( configAccount )
        assert.equal( configState.tipClaimer.toString(), initializerKeys.publicKey.toString() )
    })
    it( '#set_tip_claimer with 0 total tips succeeds', async () => {
        let configState = await paymentVaultProg.account.config.fetch( configAccount )
        const oldTipClaimer = configState.tipClaimer
        const newTipClaimer = anchor.web3.Keypair.generate()
        await paymentVaultProg.rpc.setTipClaimer(
            {
              accounts: {
                  oldTipClaimer,
                  newTipClaimer: newTipClaimer.publicKey,
                  config: configAccount,
                  signer: initializerKeys.publicKey,
                  mevPaymentAccount1,
                  mevPaymentAccount2,
                  mevPaymentAccount3,
                  mevPaymentAccount4,
                  mevPaymentAccount5,
                  mevPaymentAccount6,
                  mevPaymentAccount7,
                  mevPaymentAccount8,
              },
              signers: [initializerKeys],
            },
        )
        await assertRentExemptAccounts()
        configState = await paymentVaultProg.account.config.fetch( configAccount )
        assert.equal( configState.tipClaimer.toString(), newTipClaimer.publicKey.toString())
    })
    it( '#claim_tips `constraint = tip_claimer.key() == config.tip_claimer`', async () => {
        try {
          const wrongTipClaimer = anchor.web3.Keypair.generate().publicKey
          await paymentVaultProg.rpc.claimTips(
              {
                  accounts: {
                      claimer: initializerKeys.publicKey,
                      config: configAccount,
                      tipClaimer: wrongTipClaimer,
                      mevPaymentAccount1,
                      mevPaymentAccount2,
                      mevPaymentAccount3,
                      mevPaymentAccount4,
                      mevPaymentAccount5,
                      mevPaymentAccount6,
                      mevPaymentAccount7,
                      mevPaymentAccount8,
                  },
                  signers: [initializerKeys],
              },
          )
          assert( false )
        } catch ( e ) {
          assert.equal( e.msg, 'A raw constraint was violated' )
        }
    })
    it( '#claim_tips moves funds to correct account', async () => {
        const claimer = anchor.web3.Keypair.generate()
        const tipAmount = 1000000
        await sendTip( mevPaymentAccount1, tipAmount )
        await sendTip( mevPaymentAccount2, tipAmount )
        const totalTip = tipAmount * 2

        let configState = await paymentVaultProg.account.config.fetch( configAccount )
        const tipClaimer = configState.tipClaimer
        const tipClaimerLamportsBefore =
            (( await paymentVaultProg.provider.connection.getAccountInfo( tipClaimer )) || { lamports: 0 }).lamports
        await paymentVaultProg.rpc.claimTips(
            {
                accounts: {
                    tipClaimer,
                    claimer: claimer.publicKey,
                    config: configAccount,
                    mevPaymentAccount1,
                    mevPaymentAccount2,
                    mevPaymentAccount3,
                    mevPaymentAccount4,
                    mevPaymentAccount5,
                    mevPaymentAccount6,
                    mevPaymentAccount7,
                    mevPaymentAccount8,
                },
                signers: [ claimer ],
            },
        )

        await assertRentExemptAccounts()
        const tipClaimerLamportsAfter =
            ( await paymentVaultProg.provider.connection.getAccountInfo( tipClaimer )).lamports
        assert.equal( tipClaimerLamportsAfter - tipClaimerLamportsBefore, totalTip )
    })
    it( '#set_tip_claimer transfers funds to previous tip_claimer', async () => {
        const tipAmount = 10000000
        await sendTip( mevPaymentAccount1, tipAmount )
        await sendTip( mevPaymentAccount2, tipAmount )
        const totalTip = tipAmount * 2

        let configState = await paymentVaultProg.account.config.fetch( configAccount )
        const oldTipClaimer = configState.tipClaimer
        const oldTipClaimerBalanceBefore =
            ( await paymentVaultProg.provider.connection.getAccountInfo( oldTipClaimer )).lamports
        const newTipClaimer = anchor.web3.Keypair.generate()
        const newLeader = anchor.web3.Keypair.generate()
        await paymentVaultProg.rpc.setTipClaimer(
            {
                accounts: {
                    oldTipClaimer,
                    newTipClaimer: newTipClaimer.publicKey,
                    config: configAccount,
                    signer: newLeader.publicKey,
                    mevPaymentAccount1,
                    mevPaymentAccount2,
                    mevPaymentAccount3,
                    mevPaymentAccount4,
                    mevPaymentAccount5,
                    mevPaymentAccount6,
                    mevPaymentAccount7,
                    mevPaymentAccount8,
                },
                signers: [newLeader],
            },
        )
        await assertRentExemptAccounts()
        const oldTipClaimerBalanceAfter =
            ( await paymentVaultProg.provider.connection.getAccountInfo( oldTipClaimer )).lamports
        assert.equal( oldTipClaimerBalanceAfter, totalTip + oldTipClaimerBalanceBefore )
    })
})