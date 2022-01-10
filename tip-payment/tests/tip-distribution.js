const anchor = require( '@project-serum/anchor' )
const assert = require( 'assert' )
const { SystemProgram } = anchor.web3

const { BalanceTree } = require( '@saberhq/merkle-distributor' ).utils
const u64 = require( '@saberhq/token-utils' ).u64

const CONFIG_ACCOUNT_SEED = 'CONFIG_ACCOUNT'

const provider = anchor.AnchorProvider.local(
    null,
    {
        commitment: 'confirmed',
        preflightCommitment: 'confirmed'
    },
)
anchor.setProvider( provider )

const tipDistribution = anchor.workspace.TipDistribution

// globals
let configAccount, configBump

describe( 'tests tip_distribution', () => {
    before( async () => {
        const [acc, bump] = await anchor.web3.PublicKey.findProgramAddress(
            [Buffer.from( CONFIG_ACCOUNT_SEED, 'utf8' )],
            tipDistribution.programId,
        )
        configAccount = acc
        configBump = bump
    })

    it( '#initialize happy path', async () => {
        // given
        const initializer = await generateAccount( 100000000000000 )
        const authority = await generateAccount( 100000000000000 )
        const expiredFundsAccount = await generateAccount( 100000000000000 )
        const numEpochsValid = new anchor.BN( 3 )
        const maxValidatorCommissionBps = 1000

        // then
        try {
            await tipDistribution.rpc.initialize(
                authority.publicKey,
                expiredFundsAccount.publicKey,
                numEpochsValid,
                maxValidatorCommissionBps,
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
        const actualConfig = await tipDistribution.account.config.fetch( configAccount )
        const exptected = {
            authority: authority.publicKey,
            expiredFundsAccount: expiredFundsAccount.publicKey,
            numEpochsValid,
            maxValidatorCommissionBps,
        }
        assertConfigState( actualConfig, exptected )
    })

    it( '#init_tip_distribution_account happy path', async () => {
        // given
        const {
            validatorVoteAccount,
            maxValidatorCommissionBps: validatorCommissionBps,
            tipDistributionAccount,
            epochInfo,
        } = await setup_initTipDistributionAccount()

        // then
        try {
            await call_initTipDistributionAccount({
                merkleRootUploadAuthority: validatorVoteAccount.publicKey,
                validatorCommissionBps,
                config: configAccount,
                systemProgram: SystemProgram.programId,
                validatorVoteAccount,
                tipDistributionAccount,
            })
        } catch ( e ) {
            assert.fail( 'unexpected error: ' + e )
        }

        // expect
        const actual = await tipDistribution.account.tipDistributionAccount.fetch( tipDistributionAccount )
        const expected = {
            validatorVotePubkey: validatorVoteAccount.publicKey,
            epochCreatedAt: epochInfo.epoch,
            merkleRoot: undefined,
            merkleRootUploadAuthority: validatorVoteAccount.publicKey,
            validatorCommissionBps,
        }
        assertDistributionAccount( actual, expected )
    })

    it( '#init_tip_distribution_account fails with [ErrorCode::InvalidValidatorCommissionFeeBps]', async () => {
        // given
        const {
            validatorVoteAccount,
            maxValidatorCommissionBps,
            tipDistributionAccount,
        } = await setup_initTipDistributionAccount()

        // then
        try {
            await call_initTipDistributionAccount({
                validatorCommissionBps: maxValidatorCommissionBps + 1,
                merkleRootUploadAuthority: validatorVoteAccount.publicKey,
                config: configAccount,
                systemProgram: SystemProgram.programId,
                validatorVoteAccount,
                tipDistributionAccount,
            })
            assert.fail( 'expected exception to be thrown' )
        } catch ( e ) {
            // expect
            assert( e.errorLogs[0].includes('Validator\'s commission basis points must be greater than 0 and less than or equal to the Config account\'s max_validator_commission_bps.' ))
        }
    })

    it( '#set_merkle_root_upload_authority happy path', async () => {
        const {
            validatorVoteAccount,
            maxValidatorCommissionBps,
            tipDistributionAccount,
            epochInfo,
        } = await setup_initTipDistributionAccount()
        await call_initTipDistributionAccount({
            validatorCommissionBps: maxValidatorCommissionBps,
            merkleRootUploadAuthority: validatorVoteAccount.publicKey,
            config: configAccount,
            systemProgram: SystemProgram.programId,
            validatorVoteAccount,
            tipDistributionAccount,
        })
        const newMerkleRootUploader = anchor.web3.Keypair.generate().publicKey

        try {
            await tipDistribution.rpc.setMerkleRootUploadAuthority(
                newMerkleRootUploader,
                {
                    accounts: {
                        tipDistributionAccount,
                        validatorVoteAccount: validatorVoteAccount.publicKey,
                    },
                    signers: [validatorVoteAccount],
                },
            )
        } catch ( e ) {
            assert.fail('Unexpected error: ' + e)
        }

        const actual = await tipDistribution.account.tipDistributionAccount.fetch( tipDistributionAccount )
        const expected = {
            validatorVotePubkey: validatorVoteAccount.publicKey,
            epochCreatedAt: epochInfo.epoch,
            merkleRoot: undefined,
            merkleRootUploadAuthority: newMerkleRootUploader,
            validatorCommissionBps: maxValidatorCommissionBps,
        }
        assertDistributionAccount( actual, expected )
    })

    it( '#set_merkle_root_upload_authority fails with ErrorCode::Unauthorized', async () => {
        const {
            validatorVoteAccount,
            maxValidatorCommissionBps,
            tipDistributionAccount,
            epochInfo,
        } = await setup_initTipDistributionAccount()
        await call_initTipDistributionAccount({
            validatorCommissionBps: maxValidatorCommissionBps,
            config: configAccount,
            systemProgram: SystemProgram.programId,
            merkleRootUploadAuthority: validatorVoteAccount.publicKey,
            validatorVoteAccount,
            tipDistributionAccount,
        })
        const newMerkleRootUploader = anchor.web3.Keypair.generate().publicKey
        const unAuthedSigner = await generateAccount( 1000 )

        try {
            await tipDistribution.rpc.setMerkleRootUploadAuthority(
                newMerkleRootUploader,
                {
                    accounts: {
                        tipDistributionAccount,
                        validatorVoteAccount: unAuthedSigner.publicKey,
                    },
                    signers: [unAuthedSigner],
                },
            )
            assert.fail( 'Expected to fail' )
        } catch ( e ) {
            assert( e.errorLogs[0].includes( 'Unauthorized signer.' ))
        }

        const actual = await tipDistribution.account.tipDistributionAccount.fetch( tipDistributionAccount )
        const expected = {
            validatorVotePubkey: validatorVoteAccount.publicKey,
            epochCreatedAt: epochInfo.epoch,
            merkleRoot: undefined,
            merkleRootUploadAuthority: validatorVoteAccount.publicKey,
            validatorCommissionBps: maxValidatorCommissionBps,
        }
        assertDistributionAccount( actual, expected )
    })

    it( '#upload_merkle_root happy path', async () => {
        const {
            validatorVoteAccount,
            maxValidatorCommissionBps,
            tipDistributionAccount,
            epochInfo,
        } = await setup_initTipDistributionAccount()
        await call_initTipDistributionAccount({
            validatorCommissionBps: maxValidatorCommissionBps,
            config: configAccount,
            systemProgram: SystemProgram.programId,
            merkleRootUploadAuthority: validatorVoteAccount.publicKey,
            validatorVoteAccount,
            tipDistributionAccount,
        })

        const user0 = await generateAccount( 1000000 )
        const user1 = await generateAccount( 1000000 )
        const amount0 = new u64( 1_000_000 )
        const amount1 = new u64( 2_000_000 )

        const tree = new BalanceTree([
            { account: user0.publicKey, amount: amount0 },
            { account: user1.publicKey, amount: amount1 },
        ])


        const root = tree.getRoot()
        const maxTotalClaim = new anchor.BN( amount0 + amount1 )
        const maxNumNodes =  new anchor.BN( 2 )

        // Sleep to allow the epoch to advance
        const sched = await provider.connection.getEpochSchedule()
        await sleep( sched.slotsPerEpoch * 400 )

        try {
            await tipDistribution.rpc.uploadMerkleRoot(
                root, maxTotalClaim, maxNumNodes,
                {
                    accounts: {
                        tipDistributionAccount,
                        merkleRootUploadAuthority: validatorVoteAccount.publicKey,
                        config: configAccount,
                    },
                    signers: [validatorVoteAccount],
                },
            )
        } catch ( e ) {
            assert.fail( 'Unexpected error: ' + e )
        }

        const actual = await tipDistribution.account.tipDistributionAccount.fetch( tipDistributionAccount )
        const expected = {
            validatorVotePubkey: validatorVoteAccount.publicKey,
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
        }
        assertDistributionAccount( actual, expected )
    })

    it( '#claim happy path', async () => {
        const {
            validatorVoteAccount,
            maxValidatorCommissionBps,
            tipDistributionAccount,
        } = await setup_initTipDistributionAccount()
        await call_initTipDistributionAccount({
            validatorCommissionBps: maxValidatorCommissionBps,
            config: configAccount,
            systemProgram: SystemProgram.programId,
            merkleRootUploadAuthority: validatorVoteAccount.publicKey,
            validatorVoteAccount,
            tipDistributionAccount,
        })

        const amount0 = 1_000_000
        const amount1 = 2_000_000
        await provider.connection.confirmTransaction(
            await provider.connection.requestAirdrop(
                tipDistributionAccount, amount0 + amount1
            ),
            'confirmed',
        )
        const preBalance0 = 10000000000
        const user0 = await generateAccount( preBalance0 )
        const user1 = await generateAccount( preBalance0 )

        const tree = new BalanceTree([
            { account: user0.publicKey, amount: new u64( amount0 )},
            { account: user1.publicKey, amount: new u64( amount1 )},
        ])


        const root = tree.getRoot()
        const maxTotalClaim = new anchor.BN( amount0 + amount1 )
        const maxNumNodes =  new anchor.BN( 2 )

        // Sleep to allow the epoch to advance
        const sched = await provider.connection.getEpochSchedule()
        await sleep( sched.slotsPerEpoch * 400 )
        await tipDistribution.rpc.uploadMerkleRoot(
            root, maxTotalClaim, maxNumNodes,
            {
                accounts: {
                    tipDistributionAccount,
                    merkleRootUploadAuthority: validatorVoteAccount.publicKey,
                    config: configAccount,
                },
                signers: [validatorVoteAccount],
            },
        )

        const index = new u64( 0 )
        const amount = new u64( amount0 )
        const proof = tree.getProof( index, user0.publicKey, amount )
        let indexSeed = new anchor.BN( 0 )
        indexSeed = indexSeed.toArrayLike( Buffer, 'le', 8 )
        const [claimStatus, _bump] = await anchor.web3.PublicKey.findProgramAddress(
            [Buffer.from( 'CLAIM_STATUS', 'utf8' ), indexSeed, tipDistributionAccount.toBuffer()],
            tipDistribution.programId,
        )

        try {
            await tipDistribution.rpc.claim(
                index, amount, proof,
                {
                    accounts: {
                        config: configAccount,
                        claimStatus,
                        claimant: user0.publicKey,
                        payer: user1.publicKey,
                        systemProgram: SystemProgram.programId,
                        tipDistributionAccount,
                    },
                    signers: [user0, user1],
                }
            )
        } catch ( e ) {
            assert.fail( 'Unexpected error: ' + e )
        }

        let user0Info = await tipDistribution.provider.connection.getAccountInfo( user0.publicKey )
        assert.equal( user0Info.lamports, preBalance0 + amount0 )
    })
})


// utils

const assertConfigState = ( actual, expected ) => {
    assert.equal( actual.authority.toString(), expected.authority.toString())
    assert.equal( actual.expiredFundsAccount.toString(), expected.expiredFundsAccount.toString())
    assert.equal( actual.maxValidatorCommissionBps, expected.maxValidatorCommissionBps)
    assert.equal( actual.numEpochsValid.toString(), expected.numEpochsValid.toString())
}

const assertDistributionAccount = ( actual, expected ) => {
    assert.equal( actual.validatorVotePubkey.toString(), expected.validatorVotePubkey.toString())
    assert.equal( actual.merkleRootUploadAuthority.toString(), expected.merkleRootUploadAuthority.toString())
    assert.equal( actual.epochCreatedAt, expected.epochCreatedAt )
    assert.equal( actual.validatorCommissionBps, expected.validatorCommissionBps )

    if ( actual.merkleRoot && expected.merkleRoot ) {
        assert.equal( actual.merkleRoot.root.toString(), expected.merkleRoot.root.toString())
        assert.equal( actual.merkleRoot.maxTotalClaim.toString(), expected.merkleRoot.maxTotalClaim.toString() )
        assert.equal( actual.merkleRoot.maxNumNodes.toString(), expected.merkleRoot.maxNumNodes.toString() )
        assert.equal( actual.merkleRoot.totalFundsClaimed.toString(), expected.merkleRoot.totalFundsClaimed.toString() )
        assert.equal( actual.merkleRoot.numNodesClaimed.toString(), expected.merkleRoot.numNodesClaimed.toString() )
    } else if ( actual.merkleRoot || expected.merkleRoot ) {
        assert.fail()
    }
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

const setup_initTipDistributionAccount = async () => {
    const config = await tipDistribution.account.config.fetch( configAccount )
    const validatorVoteAccount = await generateAccount( 10000000000000 )
    const epochInfo = await provider.connection.getEpochInfo( 'confirmed' )
    let epoch = new anchor.BN( epochInfo.epoch )
    epoch = epoch.toArrayLike( Buffer, 'le', 8 )
    const [tipDistributionAccount, bump] = await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from( 'TIP_DISTRIBUTION_ACCOUNT', 'utf8' ), validatorVoteAccount.publicKey.toBuffer(), epoch],
        tipDistribution.programId,
    )

    return {
        maxValidatorCommissionBps: config.maxValidatorCommissionBps,
        validatorVoteAccount,
        tipDistributionAccount,
        bump,
        epochInfo,
    }
}

const call_initTipDistributionAccount =
    async ({ validatorCommissionBps, merkleRootUploadAuthority, config, systemProgram, validatorVoteAccount, tipDistributionAccount }) => {
    return await tipDistribution.rpc.initTipDistributionAccount(
        merkleRootUploadAuthority,
        validatorCommissionBps,
        {
            accounts: {
                config,
                systemProgram,
                validatorVoteAccount: validatorVoteAccount.publicKey,
                tipDistributionAccount,
            },
            signers: [validatorVoteAccount],
        },
    )
}

const sleep = ms => {
    return new Promise(resolve => setTimeout( resolve, ms ))
}
