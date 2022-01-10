const anchor = require( '@project-serum/anchor' )
const assert = require( 'assert' )
const { SystemProgram } = anchor.web3

const CONFIG_ACCOUNT_SEED = 'CONFIG_ACCOUNT'
const provider = anchor.Provider.local(
    undefined,
    { commitment: 'confirmed', preflightCommitment: 'confirmed' },
)
anchor.setProvider( provider )
const mevDistribution = anchor.workspace.MevDistribution

// globals
const MEV_DISTRIBUTION_ACCOUNT_SIZE = 56
let configAccount, configBump, currentAuthorityKeys
// stores successfully initialized accounts for use in different test methods
let distributionAccounts = []

describe( 'tests mev_distribution', () => {
    before( async () => {
        const [acc, bump] = await anchor.web3.PublicKey.findProgramAddress(
            [Buffer.from( CONFIG_ACCOUNT_SEED, 'utf8' )],
            mevDistribution.programId,
        )
        configAccount = acc
        configBump = bump
    })

    it( '#initialize happy path', async () => {
        // given
        const initializer = await generateAccount( 100000000000000 )
        const authority = await generateAccount( 100000000000000 )
        const distributionPot = await generateAccount( 100000000000000 )
        const maxPayerFeeBps = 1000
        
        // then
        try {
            await mevDistribution.rpc.initialize(
                authority.publicKey,
                distributionPot.publicKey,
                maxPayerFeeBps,
                configBump,
                {
                    accounts: {
                        config: configAccount,
                        systemProgram: SystemProgram.programId,
                        initializer: initializer.publicKey,
                    },
                    signers: [initializer],
                },
            )
        } catch ( e ) {
            assert.fail( 'unexpected error: ' + e )
        }

        // expect
        const actualConfig = await mevDistribution.account.config.fetch( configAccount )
        const exptected = {
            authority: authority.publicKey,
            distributionPot: distributionPot.publicKey,
            maxPayerFeeBps,
        }
        assertConfigState( actualConfig, exptected )
        currentAuthorityKeys = authority
    })

    it( '#init_distribution_account happy path', async () => {
        // given
        const {
            initializer,
            maxPayerFeeBps: payerFeeBps,
            distributionAccount,
            bump,
            epochInfo,
        } = await setup_initDistributionAccount()

        // then
        try {
            await call_initDistributionAccount({
                payerFeeBps,
                bump,
                config: configAccount,
                systemProgram: SystemProgram.programId,
                initializer,
                distributionAccount,
            })
        } catch ( e ) {
            assert.fail( 'unexpected error: ' + e )
        }

        // expect
        const actual = await mevDistribution.account.mevDistributionAccount.fetch( distributionAccount )
        const expected = {
            payer: initializer.publicKey,
            epochCreated: epochInfo.epoch,
            payerFeeSplitBps: payerFeeBps,
            bump: bump,
        }
        assertDistributionAccount( actual, expected )
        distributionAccounts.push( distributionAccount )
    })

    it( '#init_distribution_account fails with [ErrorCode::InvalidValidatorFeeSplitBps]', async () => {
        // given
        const {
            initializer,
            maxPayerFeeBps,
            distributionAccount,
            bump,
        } = await setup_initDistributionAccount()

        // then
        try {
            await call_initDistributionAccount({
                payerFeeBps: maxPayerFeeBps + 1,
                bump,
                config: configAccount,
                systemProgram: SystemProgram.programId,
                initializer,
                distributionAccount,
            })
            assert.fail( 'expected exception to be thrown' )
        } catch ( e ) {
            // expect
            assert.equal( e.msg, 'Validator\'s fee split basis points must less than or equal to max_validator_fee_bps' )
        }
    })

    it( '#transfer_distribution_account_funds `constraint = from.payer == distribution_account_payer.key()`', async () => {
        // given
        const {
            initializer,
            maxPayerFeeBps: payerFeeBps,
            distributionAccount: badDistributionAccount,
            bump,
        } = await setup_initDistributionAccount()
        await call_initDistributionAccount({
            payerFeeBps,
            bump,
            config: configAccount,
            systemProgram: SystemProgram.programId,
            initializer,
            distributionAccount: badDistributionAccount,
        })
        const goodDistributionAccount = distributionAccounts[0]
        const {
            to,
            distributionAccountPayer,
            authority,
            config,
        } = await setup_transferDistributionAccountFunds( goodDistributionAccount )

        // then
        try {
            await call_transferDistributionAccountFunds({
                config,
                from: badDistributionAccount,
                to,
                distributionAccountPayer,
                authority,
            })
            assert.fail( 'expected exception to be thrown' )
        } catch ( e ) {
            assert.equal( e.msg, 'A raw constraint was violated' )
        }
    })

    it( '#transfer_distribution_account_funds `constraint = config.distribution_pot == to.key()`', async () => {
        // given
        const from = distributionAccounts[0]
        const {
            distributionAccountPayer,
            authority,
            config,
        } = await setup_transferDistributionAccountFunds( from )
        const badTo = await generateAccount( 100000000000000 )

        // then
        try {
            await call_transferDistributionAccountFunds({
                config,
                from,
                to: badTo.publicKey,
                distributionAccountPayer,
                authority,
            })
            assert.fail( 'expected exception to be thrown' )
        } catch ( e ) {
            assert.equal( e.msg, 'A raw constraint was violated' )
        }
    })

    it( '#transfer_distribution_account_funds happy path', async () => {
        // given
        const from = distributionAccounts[0]
        const {
            to,
            toPreBalance,
            fromPreBalance,
            distributionAccountPayer,
            distributionAccountPayerPreBalance,
            authority,
            config,
        } = await setup_transferDistributionAccountFunds( from )

        // then
        try {
            await call_transferDistributionAccountFunds({
                config,
                from,
                to,
                distributionAccountPayer,
                authority,
            })
        } catch ( e ) {
            assert.fail( 'unexpected error: ' + e )
        }

        // expect
        const toPostBalance = ( await mevDistribution.provider.connection.getAccountInfo( to )).lamports
        let minRentExempt =
            await mevDistribution.provider.connection.getMinimumBalanceForRentExemption( MEV_DISTRIBUTION_ACCOUNT_SIZE )
        const expectedXferAmt = fromPreBalance - minRentExempt
        assert.equal( toPostBalance, toPreBalance + expectedXferAmt )
        const isClosed = !( await mevDistribution.provider.connection.getAccountInfo( from ))
        assert( isClosed )
        // should receive the remaining rent
        const payerPostBal =
            ( await mevDistribution.provider.connection.getAccountInfo( distributionAccountPayer )).lamports
        assert.equal( payerPostBal, distributionAccountPayerPreBalance + minRentExempt )
        // rm since it was closed
        distributionAccounts = distributionAccounts.filter( a => a.publicKey != from.publicKey )
    })
})


// utils

const assertConfigState = ( actual, expected ) => {
    assert.equal( actual.authority.toString(), expected.authority.toString())
    assert.equal( actual.maxPayerFeeBps, expected.maxPayerFeeBps)
    assert.equal( actual.distributionPot.toString(), expected.distributionPot.toString())
}

const assertDistributionAccount = ( actual, expected ) => {
    assert.equal( actual.payer.toString(), expected.payer.toString())
    assert.equal( actual.epochCreated, expected.epochCreated )
    assert.equal( actual.payerFeeSplitBps, expected.payerFeeSplitBps )
    assert.equal( actual.bump, expected.bump )
}

const generateAccount = async ( airdrop ) => {
    const account = anchor.web3.Keypair.generate()
    if ( airdrop ) {
        await provider.connection.confirmTransaction(
            await provider.connection.requestAirdrop(
                account.publicKey, airdrop
            ),
            'confirmed',
        )
    }

    return account
}

const setup_initDistributionAccount = async () => {
    const config = await mevDistribution.account.config.fetch( configAccount )
    const initializer = await generateAccount( 100000000000000 )
    const epochInfo = await provider.connection.getEpochInfo( 'finalized' )
    let epoch = new anchor.BN( epochInfo.epoch )
    epoch = epoch.toArrayLike( Buffer, 'le', 8 )
    const [distributionAccount, bump] = await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from( 'MEV_DISTRIBUTION_ACCOUNT', 'utf8' ), initializer.publicKey.toBuffer(), epoch],
        mevDistribution.programId,
    )

    return {
        maxPayerFeeBps: config.maxPayerFeeBps,
        initializer,
        distributionAccount,
        bump,
        epochInfo,
    }
}

const call_initDistributionAccount =
    async ({ payerFeeBps, bump, config, systemProgram, initializer, distributionAccount, }) => {
    return await mevDistribution.rpc.initDistributionAccount(
        payerFeeBps,
        bump,
        {
            accounts: {
                config,
                systemProgram,
                initializer: initializer.publicKey,
                distributionAccount,
            },
            signers: [initializer],
        },
    )
}

const setup_transferDistributionAccountFunds = async ( from ) => {
    if ( distributionAccounts.length == 0 ) {
        assert.fail( 'expected initialized MevDistributionAccounts' )
    }
    const config = await mevDistribution.account.config.fetch( configAccount )
    assert.equal( config.authority.toString(), currentAuthorityKeys.publicKey.toString())
    const to = config.distributionPot
    const toPreBalance = ( await mevDistribution.provider.connection.getAccountInfo( to )).lamports
    const fromPreBalance = ( await mevDistribution.provider.connection.getAccountInfo( from )).lamports
    const distributionAccountPayer =
        ( await mevDistribution.account.mevDistributionAccount.fetch( from )).payer
    const distributionAccountPayerPreBalance =
        ( await mevDistribution.provider.connection.getAccountInfo( distributionAccountPayer )).lamports
    const authority = currentAuthorityKeys

    return {
        to,
        toPreBalance,
        fromPreBalance,
        distributionAccountPayer,
        distributionAccountPayerPreBalance,
        authority,
        config: configAccount,
    }
}

const call_transferDistributionAccountFunds =
    async ({ config, authority, from, to, distributionAccountPayer }) => {
    return await mevDistribution.rpc.transferDistributionAccountFunds(
        {
            accounts: {
                config,
                authority: authority.publicKey,
                from,
                to,
                distributionAccountPayer
            },
            signers: [authority],
        },
    )
}
