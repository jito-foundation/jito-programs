import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { JitoProtecc } from "../target/types/jito_protecc";

describe("jito-protecc", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const _program = anchor.workspace.JitoProtecc as Program<JitoProtecc>;
});
