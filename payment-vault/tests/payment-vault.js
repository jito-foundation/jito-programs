const anchor = require( '@project-serum/anchor' )
const assert = require( 'assert' )
const { SystemProgram, SYSVAR_RENT_PUBKEY, Transaction } = anchor.web3

const CONFIG_ACCOUNT_LEN = 8 + 32

describe( 'tests payment_vault', () => {
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
        const args = {
          configAccountBump,
        }
        await paymentVaultProg.rpc.initialize(
            args,
            {
              accounts: {
                config: configAccount,
                initialFundingAccount: initializerKeys.publicKey,
                payer: initializerKeys.publicKey,
                systemProgram: SystemProgram.programId,
              },
              signers: [initializerKeys],
            },
        )
        const configState = await paymentVaultProg.account.config.fetch( configAccount )
        assert.equal( configState.registeredFundingAccount.toString(), initializerKeys.publicKey.toString() )
    })
    it( '#register_funding_account with no funds succeeds', async () => {
        let configState = await paymentVaultProg.account.config.fetch( configAccount )
        const oldFundingAccount = configState.registeredFundingAccount
        const newFundingAccount = anchor.web3.Keypair.generate()
        globalKeyPairStore[ newFundingAccount.publicKey ] = newFundingAccount
        await paymentVaultProg.rpc.registerFundingAccount(
            {
              accounts: {
                oldFundingAccount,
                newFundingAccount: newFundingAccount.publicKey,
                config: configAccount,
                rent: SYSVAR_RENT_PUBKEY,
                signer: initializerKeys.publicKey,
              },
              signers: [initializerKeys],
            },
        )
        const minRentExempt = await paymentVaultProg.provider.connection.getMinimumBalanceForRentExemption( CONFIG_ACCOUNT_LEN )
        const configInfo = await paymentVaultProg.provider.connection.getAccountInfo( configAccount )
        assert.equal( configInfo.lamports, minRentExempt )
        configState = await paymentVaultProg.account.config.fetch( configAccount )
        assert.equal( configState.registeredFundingAccount.toString(), newFundingAccount.publicKey.toString())
    })
    it( '#claim_funds `constraint = registered_funding_account.key() == config.registered_funding_account`', async () => {
        try {
          const wrongFundingAccount = anchor.web3.Keypair.generate().publicKey
          await paymentVaultProg.rpc.claimFunds(
              {
                  accounts: {
                      claimer: initializerKeys.publicKey,
                      config: configAccount,
                      registeredFundingAccount: wrongFundingAccount,
                      rent: SYSVAR_RENT_PUBKEY,
                  },
                  signers: [initializerKeys],
              },
          )
          assert( false )
        } catch ( e ) {
          assert.equal( e.msg, 'A raw constraint was violated' )
        }
    })
    it( '#claim_funding moves funds to correct account', async () => {
        const claimer = anchor.web3.Keypair.generate()
        const searcherKP = anchor.web3.Keypair.generate()
        const airDrop = 100000000000000
        await provider.connection.confirmTransaction(
            await provider.connection.requestAirdrop(
                searcherKP.publicKey, airDrop
            ),
            'confirmed',
        )
        const tipAmount = airDrop / 2
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
        let configState = await paymentVaultProg.account.config.fetch( configAccount )
        const registeredFundingAccount = configState.registeredFundingAccount
        await paymentVaultProg.rpc.claimFunds(
            {
                accounts: {
                    registeredFundingAccount,
                    claimer: claimer.publicKey,
                    config: configAccount,
                    rent: SYSVAR_RENT_PUBKEY,
                },
              signers: [ claimer ],
            },
        )
        const minRentExempt = await paymentVaultProg.provider.connection.getMinimumBalanceForRentExemption( CONFIG_ACCOUNT_LEN )
        configInfo = await paymentVaultProg.provider.connection.getAccountInfo( configAccount )
        // check that account is still rent exempt
        assert.equal( configInfo.lamports, minRentExempt )
        const fundingInfo = await paymentVaultProg.provider.connection.getAccountInfo( registeredFundingAccount )
        assert.equal( fundingInfo.lamports, tipAmount )
    })
})