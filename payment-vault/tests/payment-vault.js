const anchor = require( '@project-serum/anchor' )
const { TOKEN_PROGRAM_ID, Token } = require( '@solana/spl-token' )
const assert = require( 'assert' )
const { SystemProgram, SYSVAR_RENT_PUBKEY } = anchor.web3

// TODO(seg): add tests for expected Unauthorized errors specifically the `auth_config_account` access_constrol attr
describe( 'tests payment_vault', () => {
  const provider = anchor.Provider.env()
  anchor.setProvider( provider )
  const paymentVaultProg = anchor.workspace.PaymentVault
  const initializerKeys = anchor.web3.Keypair.generate()
  const mintAuthority = anchor.web3.Keypair.generate()

  let configAccount, configAccountBump, feeAccount, feeAccountBump, tipAccount, tipAccountBump, mint
  before( async () => {
    const [_configAccount, _configAccountBump] = await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from( 'GOKU_CONFIG_ACCOUNT_SEED', 'utf8' )],
        paymentVaultProg.programId,
    )
    configAccount = _configAccount
    configAccountBump = _configAccountBump

    const [_feeAccount, _feeAccountBump] = await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from( 'VEGETA_FEE_ACCOUNT_SEED', 'utf8' )],
        paymentVaultProg.programId,
    )
    feeAccount = _feeAccount
    feeAccountBump = _feeAccountBump

    const [_tipAccount, _tipAccountBump] = await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from( 'RYU_TIP_ACCOUNT_SEED', 'utf8' )],
        paymentVaultProg.programId,
    )
    tipAccount = _tipAccount
    tipAccountBump = _tipAccountBump

    await provider.connection.confirmTransaction(
        await provider.connection.requestAirdrop(
            initializerKeys.publicKey, 100000000000000
        ),
        'confirmed',
    )
    mint = await Token.createMint(
        provider.connection,
        initializerKeys,
        mintAuthority.publicKey,
        null,
        12,
        TOKEN_PROGRAM_ID,
    )
  })

  it( '#initialize fails with bad seed for config account', async () => {
    const keys = anchor.web3.Keypair.generate()
    const [_configAccount, _configAccountBump] = await anchor.web3.PublicKey.findProgramAddress(
        [keys.publicKey.toBuffer()],
        paymentVaultProg.programId,
    )
    const args = {
      feeBps: new anchor.BN(10),
      configAccountBump: _configAccountBump,
      feeAccountBump,
      tipAccountBump,
    }
    try {
      await paymentVaultProg.rpc.initialize(
          args,
          {
            accounts: {
              feeAccount,
              tipAccount,
              config: _configAccount,
              mint: mint.publicKey,
              payer: initializerKeys.publicKey,
              tokenProgram: TOKEN_PROGRAM_ID,
              systemProgram: SystemProgram.programId,
              rent: SYSVAR_RENT_PUBKEY,
            },
            signers: [initializerKeys],
          },
      )
      assert(false )
    } catch ( e ) {
      assert( e.logs )
      const errLog = e
          .logs
          .find( log =>
              log.includes( 'Could not create program address with signer seeds: Provided seeds do not result in a valid address' )
          )
      assert( errLog )
    }
  })

  it( '#initialize fails with poorly derived tip_account addr', async () => {
    const keys = anchor.web3.Keypair.generate()
    await provider.connection.confirmTransaction(
        await provider.connection.requestAirdrop(
            keys.publicKey, 100000000000000
        ),
        'confirmed',
    )
    const _tipAccount = await mint.createAccount( keys.publicKey )
    const badTipAccountBump = 77
    const args = {
      feeBps: new anchor.BN(10),
      configAccountBump,
      feeAccountBump,
      tipAccountBump: badTipAccountBump,
    }
    try {
      await paymentVaultProg.rpc.initialize(
          args,
          {
            accounts: {
              feeAccount,
              tipAccount: _tipAccount,
              mint: mint.publicKey,
              config: configAccount,
              payer: keys.publicKey,
              tokenProgram: TOKEN_PROGRAM_ID,
              systemProgram: SystemProgram.programId,
              rent: SYSVAR_RENT_PUBKEY,
            },
            signers: [keys],
          },
      )
      assert(false )
    } catch ( e ) {
      const errLog = e
          .logs
          .find( log =>
              log.includes( 'Could not create program address with signer seeds: Provided seeds do not result in a valid address' )
          )
      assert( errLog )
    }
  })

  it( '#initialize happy path', async () => {
    const args = {
      feeBps: new anchor.BN(10),
      configAccountBump,
      feeAccountBump,
      tipAccountBump,
    }
    await paymentVaultProg.rpc.initialize(
        args,
        {
          accounts: {
            feeAccount,
            tipAccount,
            mint: mint.publicKey,
            config: configAccount,
            payer: initializerKeys.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
            rent: SYSVAR_RENT_PUBKEY,
          },
          signers: [initializerKeys],
        },
    )

    const configState = await paymentVaultProg.account.config.fetch( configAccount )
    assert.equal( configState.feeAccountPk.toString(), feeAccount.toString())
    assert.equal( configState.feeBps.toString(), args.feeBps.toString())
    assert.equal( configState.tipAccountPk.toString(), tipAccount.toString())
    const mintInfo = await mint.getMintInfo()
    assert.equal( configState.decimals, mintInfo.decimals )

    // check Config account is the new owner of tipAccount
    const tipAccInfo = await mint.getAccountInfo( tipAccount )
    assert.equal( tipAccInfo.owner.toString(), configAccount.toString())
    // check feeAccount owner is untouched
    const feeAccInfo = await mint.getAccountInfo( feeAccount )
    assert.equal( feeAccInfo.owner.toString(), configAccount.toString())
  })

  it( '#initialize fails to init config_account twice', async () => {
    const args = {
      feeBps: new anchor.BN(10),
      configAccountBump,
      feeAccountBump,
      tipAccountBump,
    }
    try {
      await paymentVaultProg.rpc.initialize(
          args,
          {
            accounts: {
              feeAccount,
              tipAccount,
              mint: mint.publicKey,
              config: configAccount,
              payer: initializerKeys.publicKey,
              tokenProgram: TOKEN_PROGRAM_ID,
              systemProgram: SystemProgram.programId,
              rent: SYSVAR_RENT_PUBKEY,
            },
            signers: [initializerKeys],
          },
      )
      assert( false )
    } catch ( e ) {
      assert( e.logs )
      const errLog = e
          .logs
          .find( log =>
              log.includes( 'already in us' )
          )
      assert( errLog )
    }
  })

  it( '#claim_tips happy path', async () => {
    const searcherKeys = anchor.web3.Keypair.generate()
    await provider.connection.confirmTransaction(
        await provider.connection.requestAirdrop(
            searcherKeys.publicKey, 100000000000000
        ),
        'confirmed',
    )
    // mint tokens to searcher
    const searcherTokenAcc = await mint.createAccount( searcherKeys.publicKey )
    const initialSearcherBal = 100000000
    await mint.mintTo( searcherTokenAcc, mintAuthority, [], initialSearcherBal )
    // xfer tokens from searcher to tip_account
    const tippedAmount = initialSearcherBal / 2
    mint.transfer( searcherTokenAcc, tipAccount, searcherKeys, [], tippedAmount )
    // set up validator account
    const validatorKeys = anchor.web3.Keypair.generate()
    await provider.connection.confirmTransaction(
        await provider.connection.requestAirdrop(
            validatorKeys.publicKey, 100000000000000
        ),
        'confirmed',
    )
    const validatorTokenAccount = await mint.createAccount( validatorKeys.publicKey )
    // sanity check
    assert.equal( mint.getAccountInfo( validatorTokenAccount ).amount || 0, 0 )
    const args = {
      configBump: configAccountBump,
    }
    const configState = await paymentVaultProg.account.config.fetch( configAccount )
    await paymentVaultProg.rpc.claimTips(
        args,
        {
          accounts: {
            config: configAccount,
            tokenProgram: TOKEN_PROGRAM_ID,
            tipAccount,
            feeAccount,
            validatorTokenAccount,
            validator: validatorKeys.publicKey,
          },
          signers: [validatorKeys],
        },
    )
    const feeRate = configState.feeBps / 10000
    const validatorBalance = ( await mint.getAccountInfo( validatorTokenAccount )).amount
    assert.equal( validatorBalance.toString(), (tippedAmount - ( tippedAmount * feeRate )).toString())
    const feeAccountBalance = ( await mint.getAccountInfo( feeAccount )).amount
    assert.equal( feeAccountBalance.toString(), ( tippedAmount * feeRate ).toString())
  })

  // TODO(seg)
  // it( '#claim_tips constraint[tip_account.key() == config.tip_account_pk]', async () => {
  //   assert.fail()
  // })
  //
  // it( '#claim_tips constraint[validator_token_account.owner == *validator_token_account_owner.key]', async () => {
  //   assert.fail()
  // })
  //
  // it( '#claim_tips constraint[*claim_authority.key == vault_account.claim_authority_pk]', async () => {
  //   assert.fail()
  // })
})