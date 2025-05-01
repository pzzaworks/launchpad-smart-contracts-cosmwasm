import { deployContract } from './deployContract';
import { MerkleTree } from 'merkletreejs';
import Crypto from 'crypto-js';

export const deploySaleContract = async (client: any, ownerAddress: any) => {
    const whitelist: any = [
        "",
        ""
    ].map((v) => Crypto.SHA256(v));
    
    const tree = new MerkleTree(whitelist, Crypto.SHA256);
    const root = tree.getRoot().toString('hex');
    const proof = tree.getProof(whitelist[0]);
    
    const current_block_timestamp = Number(Math.floor(new Date((await client.getBlock()).header.time).getTime() / 1000));
    
    const saleProperties = {
        label: "Sale Contract",
        properties: {
            owner: ownerAddress,
            stake_controller: "",
            payment_token: "",
            sale_token_decimals: (6).toString(),
            sale_token_price: (1*10**2).toString(),
            min_allocation: (1*10**2).toString(),
            total_allocation: (100000*10**6).toString(),
            fcfs_multiplier: (1500).toString(),
            fcfs_allocation: (0).toString(), // 0 for unlimited allocation
            status: {
                register_paused: false,
                staker_paused: false,
                fcfs_paused: false,
            },
            dates: {
                register_start: (current_block_timestamp).toString(),
                register_end: (current_block_timestamp + 7 * 24 * 60 * 60).toString(),
                staker_start: (current_block_timestamp + 7 * 24 * 60 * 60).toString(),
                staker_end: (current_block_timestamp + 14 * 24 * 60 * 60).toString(),
                fcfs_start: (current_block_timestamp + 14 * 24 * 24 * 60).toString(),
                fcfs_end: (current_block_timestamp + 21 * 24 * 60 * 60).toString(),
            },
            whitelist_properties: {
                whitelist_merkle_root: ("").toString(),
                whitelisted_user_count: (0).toString(),
                whitelisted_user_allocation: (100*10**6).toString(),
            },
        }
    };

    const saleAddress = await deployContract(
        "./artifacts/sale.wasm", 
        saleProperties, 
        client, 
        ownerAddress
    );

    return saleAddress;
}