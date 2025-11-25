import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
// Asumo que el nombre generado es EscrowNft (PascalCase de escrow_nft)
import { EscrowNft } from "../target/types/escrow_nft"; 
import { Keypair, PublicKey } from "@solana/web3.js";
import { BN} from "bn.js";
import { createMint, getAssociatedTokenAddress, getOrCreateAssociatedTokenAccount, mintTo, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import {createUmi} from "@metaplex-foundation/umi-bundle-defaults";
import { createSignerFromKeypair,generateSigner,signerIdentity, KeypairSigner } from "@metaplex-foundation/umi";
import {createV1, fetchAssetsByOwner, MPL_CORE_PROGRAM_ID, mplCore} from "@metaplex-foundation/mpl-core";
import {fromWeb3JsKeypair, fromWeb3JsPublicKey,toWeb3JsPublicKey} from "@metaplex-foundation/umi-web3js-adapters";
import { ASSOCIATED_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/utils/token";
import { SYSTEM_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/native/system";
import { expect } from "chai";


// El nombre del programa es 'escrow_nft'
describe("escrow_nft", () => {
  
  let provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  // Usa el nombre del workspace (escrowNft o escrow_nft)
  const program = anchor.workspace.escrowNft as Program<EscrowNft>; 
  const connection = provider.connection;

  const token_program = TOKEN_PROGRAM_ID;
  const payer = provider.wallet as anchor.Wallet;

  let maker: Keypair;
  let buyer: Keypair;

  let mint_sol: PublicKey;
  let maker_ata_sol: PublicKey;
  let buyer_ata_sol: PublicKey;
  let vault_ata_sol: PublicKey;
  
  // Novedad: ATAs para el NFT (necesarias en take y refund)
  let maker_asset_ata: PublicKey; 
  let taker_asset_ata: PublicKey; 
  
  let escrowPDA: PublicKey;
  let escrowBump: number;

  //MPL Core asset
  let assetAddress: PublicKey;
  let umi: any;
  let asset: KeypairSigner;


  before("Setup" , async () => {
    maker = Keypair.generate();
    buyer = Keypair.generate();

    // 1. Airdrop de SOL para fees
    const makerAirdropSignature = await connection.requestAirdrop(
    maker.publicKey,
    1 * anchor.web3.LAMPORTS_PER_SOL
    );

    // Usamos await para esperar la confirmación de la transacción.
    await connection.confirmTransaction({
        signature: makerAirdropSignature,
        blockhash: (await provider.connection.getLatestBlockhash()).blockhash,
        lastValidBlockHeight: (await provider.connection.getLatestBlockhash()).lastValidBlockHeight
    });

    // Repetimos el patrón para el buyer.
    const buyerAirdropSignature = await connection.requestAirdrop(
        buyer.publicKey,
        1 * anchor.web3.LAMPORTS_PER_SOL
    );

    await connection.confirmTransaction({
        signature: buyerAirdropSignature,
        blockhash: (await provider.connection.getLatestBlockhash()).blockhash,
        lastValidBlockHeight: (await provider.connection.getLatestBlockhash()).lastValidBlockHeight,
    });

    // 2. Mint_sol (Token SPL de Pago, ej. USDC)
    mint_sol = await createMint(
      connection,
      payer.payer, 
      payer.publicKey, 
      null, 
      6,
      undefined,
      undefined,
      token_program
    );
    
    // 3. Creación del NFT/Asset de Metaplex Core
    umi = createUmi(connection);
    const umiMaker = fromWeb3JsKeypair(payer.payer); 
    const umiSigner = createSignerFromKeypair(umi,umiMaker); 
    umi.use(signerIdentity(umiSigner));
    umi.use(mplCore());

    asset = generateSigner(umi);

    await createV1(umi,{
        asset,
        name: "Test NFT",
        uri: "",
        owner: fromWeb3JsPublicKey(maker.publicKey),
      }).sendAndConfirm(umi);

    assetAddress = toWeb3JsPublicKey(asset.publicKey);


    // 4. Derivación del PDA y ATAs
    const seed = new anchor.BN(2);
    [escrowPDA,escrowBump] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("escrow"),
        seed.toArrayLike(Buffer,"le",8),
      ],
      program.programId
    );    

    vault_ata_sol = await getAssociatedTokenAddress(
      mint_sol,
      escrowPDA,
      true, 
      token_program
    );

    // Derivación de las ATAs de NFT
    maker_asset_ata = await getAssociatedTokenAddress(assetAddress, maker.publicKey, false, token_program);
    taker_asset_ata = await getAssociatedTokenAddress(assetAddress, buyer.publicKey, false, token_program);

    // Creación de las ATAs de Token SPL de Pago (Maker y Buyer)
    maker_ata_sol= (await getOrCreateAssociatedTokenAccount(connection, provider.wallet.payer, mint_sol, maker.publicKey)).address;
    buyer_ata_sol= (await getOrCreateAssociatedTokenAccount(connection, provider.wallet.payer, mint_sol, buyer.publicKey)).address;
  
    // Airdrop tokens de pago a maker y buyer
    await mintTo(connection, payer.payer, mint_sol, maker_ata_sol, payer.publicKey, 100*10**6, [payer.payer], undefined, token_program);
    await mintTo(connection, payer.payer, mint_sol, buyer_ata_sol, payer.publicKey, 100*10**6, [payer.payer], undefined, token_program);


    console.log("Setup complete. Asset Mint:", assetAddress.toBase58());
  });


  // 5. Test Initialize
  it("Is initialized!", async () => {
    
    const seed  = new BN(2);
    const escrowBeforeInitialization = await  connection.getAccountInfo(escrowPDA);
    expect(escrowBeforeInitialization).to.be.null;

    const tx = await program.methods.initialize(seed, new BN(3)).accountsPartial({
      maker: maker.publicKey,
      mintSol: mint_sol,
      asset: assetAddress,
      vault: vault_ata_sol,
      escrow: escrowPDA,
      makerAtaSol: maker_ata_sol,
      associatedTokenProgram: ASSOCIATED_PROGRAM_ID,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SYSTEM_PROGRAM_ID,
      mplCoreProgram: MPL_CORE_PROGRAM_ID
    }).signers([maker]).rpc();
    console.log("Initialize transaction signature", tx);
    
    const escrowAfterInitialization = await connection.getAccountInfo(escrowPDA);
    expect(escrowAfterInitialization).to.not.be.null;
    
    // Asumo que tu struct se llama EscrowState en la IDL
    const escrowState = await program.account.escrowState.fetch(escrowPDA);
    expect(escrowState.price.toString()).to.equal((new BN(3)).toString());
    expect(escrowState.maker.toBase58()).to.equal(maker.publicKey.toBase58());
  });

  // 6. Test Make (Depositar NFT)
  it("MAKE Listing: Deposits NFT to Escrow", async () => {
    const amount = new BN(4);
    const seed  = new BN(2);
    
    // Cambio de listNft a MAKE
    const tx = await program.methods.make(amount, seed).accountsPartial({ 
      maker: maker.publicKey,
      asset: assetAddress,
      vault: vault_ata_sol,
      escrow: escrowPDA,
      associatedTokenProgram: ASSOCIATED_PROGRAM_ID,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SYSTEM_PROGRAM_ID,
      mplCoreProgram: MPL_CORE_PROGRAM_ID
    }).signers([maker]).rpc()

    console.log("MAKE transaction signature: ", tx);

    // Verifica que el NFT esté en el escrow
    const assetsByEscrow = await fetchAssetsByOwner(umi, escrowPDA.toString(), {
     skipDerivePlugins: false, 
   })
    expect(assetsByEscrow.length).to.equal(1);
    expect(assetsByEscrow[0].publicKey.toString()).to.equal(asset.publicKey.toString());
  });
  /*
  // 7. Test Take (Comprar/Intercambio Atómico)
  it("TAKE Listing: Executes atomic NFT-for-SPL exchange", async() => {
    const seed = new BN(2);
    
    // Cambio de buyNft a TAKE
    const tx = await program.methods.take(seed).accountsPartial({
      taker: buyer.publicKey, 
      maker: maker.publicKey,
      mintSol: mint_sol,
      asset: assetAddress,
      vault: vault_ata_sol,
      escrow: escrowPDA,
      makerAtaSol: maker_ata_sol,
      takerAtaSol: buyer_ata_sol,
      associatedTokenProgram: ASSOCIATED_PROGRAM_ID,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SYSTEM_PROGRAM_ID,
      mplCoreProgram: MPL_CORE_PROGRAM_ID
    }).signers([buyer]).rpc();

    console.log("TAKE transaction signature: ", tx);
    
    // Verificación 1: El NFT es propiedad del Taker
    const assetsByBuyer = await fetchAssetsByOwner(umi, buyer.publicKey.toString(), {
      skipDerivePlugins: false, 
    })
    expect(assetsByBuyer.length).to.equal(1);
    
    // Verificación 2: La cuenta de escrow ha sido cerrada
    const escrowAfterTake = await connection.getAccountInfo(escrowPDA);
    expect(escrowAfterTake).to.be.null;

  });
  */

  // 8. Test Refund (Cancelación) - ¡Comentar este si ejecutas TAKE, o viceversa!
  it("REFUND Listing: Maker cancels and recovers NFT", async() => {
    
    const seed  = new BN(2);
    
    // Cambio de unlist a REFUND
    const refundtx = await program.methods.refund(seed).accountsPartial({
      maker: maker.publicKey,
      asset: assetAddress,
      escrow: escrowPDA,
      associatedTokenProgram: ASSOCIATED_PROGRAM_ID,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SYSTEM_PROGRAM_ID,
      mplCoreProgram: MPL_CORE_PROGRAM_ID
    }).signers([maker]).rpc();

    console.log("REFUND transaction signature: ",refundtx);
    
    // Verificación 1: El NFT es propiedad del Maker
    const assetsByMaker = await fetchAssetsByOwner(umi, maker.publicKey.toString(), {
      skipDerivePlugins: false, 
    })
    expect(assetsByMaker.length).to.equal(1);
    
    // Verificación 2: La cuenta de escrow ha sido cerrada
    const escrowAfterRefund = await connection.getAccountInfo(escrowPDA);
    expect(escrowAfterRefund).to.be.null;
  });
});