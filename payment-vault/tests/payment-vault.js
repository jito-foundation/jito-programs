const anchor = require( '@project-serum/anchor' )
const assert = require( 'assert' )
const { SystemProgram, Transaction } = anchor.web3

const CONFIG_ACCOUNT_LEN = 8 + 32

describe( 'tests payment_vault', () => {
    const tipConfigAccount = async ( tipAmount ) => {
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
                toPubkey: configAccount,
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
    const globalKeyPairStore = {}
    let configAccount, configAccountBump
    before( async () => {
        const [_configAccount, _configAccountBump] = await anchor.web3.PublicKey.findProgramAddress(
            [Buffer.from( 'CONFIG_ACCOUNT', 'utf8' )],
            paymentVaultProg.programId,
        )
        configAccount = _configAccount
        configAccountBump = _configAccountBump

        await provider.connection.confirmTransaction(
            await provider.connection.requestAirdrop(
                initializerKeys.publicKey, 100000000000000
            ),
            'confirmed',
        )
    })
    it( '#initialize happy path', async () => {
        await paymentVaultProg.rpc.initialize(
            configAccountBump,
            {
              accounts: {
                config: configAccount,
                initialTipClaimer: initializerKeys.publicKey,
                payer: initializerKeys.publicKey,
                systemProgram: SystemProgram.programId,
              },
              signers: [initializerKeys],
            },
        )
        const configState = await paymentVaultProg.account.config.fetch( configAccount )
        assert.equal( configState.tipClaimer.toString(), initializerKeys.publicKey.toString() )
    })
    it( '#change_tip_claimer with no funds succeeds', async () => {
        let configState = await paymentVaultProg.account.config.fetch( configAccount )
        const oldTipClaimer = configState.tipClaimer
        const newTipClaimer = anchor.web3.Keypair.generate()
        globalKeyPairStore[ newTipClaimer.publicKey ] = newTipClaimer
        await paymentVaultProg.rpc.changeTipClaimer(
            {
              accounts: {
                oldTipClaimer,
                newTipClaimer: newTipClaimer.publicKey,
                config: configAccount,
                signer: initializerKeys.publicKey,
              },
              signers: [initializerKeys],
            },
        )
        const minRentExempt = await paymentVaultProg.provider.connection.getMinimumBalanceForRentExemption( CONFIG_ACCOUNT_LEN )
        const configInfo = await paymentVaultProg.provider.connection.getAccountInfo( configAccount )
        assert.equal( configInfo.lamports, minRentExempt )
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
        await tipConfigAccount( tipAmount )
        let configState = await paymentVaultProg.account.config.fetch( configAccount )
        const tipClaimer = configState.tipClaimer
        await paymentVaultProg.rpc.claimTips(
            {
                accounts: {
                    tipClaimer,
                    claimer: claimer.publicKey,
                    config: configAccount,
                },
              signers: [ claimer ],
            },
        )
        const minRentExempt = await paymentVaultProg.provider.connection.getMinimumBalanceForRentExemption( CONFIG_ACCOUNT_LEN )
        configInfo = await paymentVaultProg.provider.connection.getAccountInfo( configAccount )
        // check that account is still rent exempt
        assert.equal( configInfo.lamports, minRentExempt )
        const tipClaimerLamports = ( await paymentVaultProg.provider.connection.getAccountInfo( tipClaimer )).lamports
        assert.equal( tipClaimerLamports, tipAmount )
    })
    it( '#change_tip_claimer transfers funds to previous tip_claimer', async () => {
        const tipAmount = 100000
        await tipConfigAccount( tipAmount )
        let configState = await paymentVaultProg.account.config.fetch( configAccount )
        const oldTipClaimer = configState.tipClaimer
        const oldTipClaimerBalance = ( await paymentVaultProg.provider.connection.getAccountInfo( oldTipClaimer )).lamports
        const newTipClaimer = anchor.web3.Keypair.generate()
        const newLeader = anchor.web3.Keypair.generate()
        await paymentVaultProg.rpc.changeTipClaimer(
            {
                accounts: {
                    oldTipClaimer,
                    newTipClaimer: newTipClaimer.publicKey,
                    config: configAccount,
                    signer: newLeader.publicKey,
                },
                signers: [newLeader],
            },
        )
        const minRentExempt = await paymentVaultProg.provider.connection.getMinimumBalanceForRentExemption( CONFIG_ACCOUNT_LEN )
        configInfo = await paymentVaultProg.provider.connection.getAccountInfo( configAccount )
        // check that account is still rent exempt
        assert.equal( configInfo.lamports, minRentExempt )
        const updatedOldTipClaimerBalance = ( await paymentVaultProg.provider.connection.getAccountInfo( oldTipClaimer )).lamports
        assert.equal( updatedOldTipClaimerBalance, tipAmount + oldTipClaimerBalance )
    })
})