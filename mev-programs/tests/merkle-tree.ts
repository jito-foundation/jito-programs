import { createHash } from "crypto";

export const convertBufProofToNumber = (buffers: Buffer[]): number[][] => {
  return buffers.reduce((prev, cur) => {
    const numArray = new Array(cur.length);
    for (let i = 0; i < cur.length; i = i + 1) numArray[i] = cur[i];
    prev.push(numArray);
    return prev;
  }, [] as number[][]);
};

export class MerkleTree {
  leafs: Array<Buffer>;
  layers: Array<Array<Buffer>>;

  constructor(leafs: Array<Buffer>) {
    this.leafs = leafs.slice();
    this.layers = [];

    let hashes = this.leafs.map(MerkleTree.nodeHash);
    while (hashes.length > 0) {
      // console.log("Hashes", this.layers.length, hashes);
      this.layers.push(hashes.slice());
      if (hashes.length === 1) break;
      hashes = hashes.reduce((acc, cur, idx, arr) => {
        if (idx % 2 === 0) {
          const nxt = arr[idx + 1];
          acc.push(MerkleTree.internalHash(cur, nxt));
        }
        return acc;
      }, Array<Buffer>());
    }
  }

  static sha256(...args: Buffer[]): Buffer {
    const hash = createHash("sha256");
    args.forEach((x) => hash.update(x));
    return hash.digest();
  }

  static nodeHash(data: Buffer): Buffer {
    //normal method
    //return MerkleTree.sha256(Buffer.from([0x00]), data);

    //jito method since stuff comes in already hashed, see https://github.com/jito-labs/jito-solana/blob/efa56db23943ae1b2a940f1e215aa75edc99be16/merkle-tree/src/merkle_tree.rs#L57
    return MerkleTree.sha256(Buffer.from([0x00]), MerkleTree.sha256(data));
  }

  static internalHash(first: Buffer, second: Buffer | undefined): Buffer {
    if (!second) return first;
    const [fst, snd] = [first, second].sort(Buffer.compare);

    return MerkleTree.sha256(Buffer.from([0x01]), fst, snd);
  }

  getRoot(): Buffer {
    return this.layers[this.layers.length - 1][0];
  }

  getProof(idx: number): Buffer[] {
    return this.layers.reduce((proof, layer) => {
      const sibling = idx ^ 1;
      if (sibling < layer.length) {
        proof.push(layer[sibling]);
      }

      idx = Math.floor(idx / 2);

      return proof;
    }, []);
  }

  getHexRoot(): string {
    return this.getRoot().toString("hex");
  }

  getHexProof(idx: number): string[] {
    return this.getProof(idx).map((el) => el.toString("hex"));
  }

  verifyProof(idx: number, proof: Buffer[], root: Buffer): boolean {
    let pair = MerkleTree.nodeHash(this.leafs[idx]);
    for (const item of proof) {
      pair = MerkleTree.internalHash(pair, item);
    }

    return pair.equals(root);
  }

  static verifyClaim(leaf: Buffer, proof: Buffer[], root: Buffer): boolean {
    let pair = MerkleTree.nodeHash(leaf);
    for (const item of proof) {
      pair = MerkleTree.internalHash(pair, item);
    }

    return pair.equals(root);
  }
}
