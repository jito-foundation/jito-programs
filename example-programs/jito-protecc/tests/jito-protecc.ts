import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { JitoProtecc } from "../target/types/jito_protecc";

describe("jito-protecc", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.JitoProtecc as Program<JitoProtecc>;

  it("Is initialized!", async () => {
    // Add your test here.
    const tx = await program.methods.initialize().rpc();
    console.log("Your transaction signature", tx);
  });
});
