import { Testnet, Mainnet, NibiruTxClient, NibiruQuerier, newSignerFromMnemonic } from '@nibiruchain/nibijs';
const { DirectSecp256k1HdWallet } = require("@cosmjs/proto-signing");
const { SigningCosmWasmClient } = require("@cosmjs/cosmwasm-stargate");
import { GasPrice } from "@cosmjs/stargate";

import dotenv from 'dotenv';
dotenv.config();

import { interactStakeContract } from "./interactStake";
import { interactSaleContract } from "./interactSale";
import { interactFaucetContract } from "./interactFaucet";
import { interactVestingContract } from './interactVesting';

const deploy = async () => {
    console.log("Starting interact...");
    
    // For Nibiru ->
    // const chain = Testnet();
    // const querier = await NibiruQuerier.connect(chain.endptTm);
    // const signer = await newSignerFromMnemonic(process.env.MNEMONIC!);
    // const txClient = await NibiruTxClient.connectWithSigner(chain.endptTm, signer);
    // const client = txClient.wasmClient;
    // const [owner] = await signer.getAccounts();
    
    // For Xion ->
    const chainInfo = {
      rpc: "https://testnet-rpc.xion-api.com",
      rest: "https://testnet-api.xion-api.com",
      chainId: "xion-testnet-1",
      bech32Prefix: "xion"
    };
    
    const mnemonic = process.env.MNEMONIC!;
    const wallet = await DirectSecp256k1HdWallet.fromMnemonic(mnemonic, {
      prefix: chainInfo.bech32Prefix
    });
    const [owner] = await wallet.getAccounts();
    const gasPrice = GasPrice.fromString("0.0025uxion");
    const client = await SigningCosmWasmClient.connectWithSigner(chainInfo.rpc, wallet, { gasPrice });

    // await interactStakeContract(client, owner.address);
    // await interactSaleContract(client, owner.address);k
    // await interactFaucetContract(client, owner.address);
    await interactVestingContract(client, owner.address);

    console.log("Interact complete!");
};

deploy().catch(console.error);