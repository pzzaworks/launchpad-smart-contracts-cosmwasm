import { Testnet, NibiruTxClient, NibiruQuerier, newSignerFromMnemonic } from '@nibiruchain/nibijs';
import fs from 'fs';
import dotenv from 'dotenv'
import { MerkleTree } from 'merkletreejs'
import Crypto from 'crypto-js'

dotenv.config();

const PRESALE_CONTRACT_ADDRESS = ""

const updateMerkleRooot = async () => {
  const chain = Testnet()
      
  const signer = await newSignerFromMnemonic(process.env.MNEMONIC) // Signer: in-practice
  const txClient = await NibiruTxClient.connectWithSigner(
    chain.endptTm,
    signer
  )
  const client = txClient.wasmClient;
  
  const [firstAccount] = await signer.getAccounts();
  const senderAddress = firstAccount.address;
  
  const whitelist = [
    "",
    ""
  ].map((v) => Crypto.SHA256(v));

  const tree = new MerkleTree(whitelist, Crypto.SHA256)
  const root = tree.getRoot().toString('hex')

  // (3)
  console.log('Merkle Root:', root);

  // (4)
  // fs.writeFileSync("tree.json", JSON.stringify(tree.dump()));
  console.log('Merkle Tree', tree.toString())

  const proof = tree.getProof(whitelist[0])
  console.log("Proof:", proof)
  const result = await client.execute(senderAddress, PRESALE_CONTRACT_ADDRESS, {
    set_merkle_root: {
      merkle_root: root
    }
  }, "auto")
  console.log(result);
};

updateMerkleRooot().catch(console.error);
