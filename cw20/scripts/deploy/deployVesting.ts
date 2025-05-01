import { deployContract } from './deployContract';

export const deployVestingContract = async (client: any, ownerAddress: any) => {
    const current_block_timestamp = Number(Math.floor(new Date((await client.getBlock()).header.time).getTime() / 1000));
    
    const vestingProperties = {
        label: "Vesting Contract",
        properties: {
            owner: ownerAddress,
            token: "",
            fee_address: ownerAddress,
            total_token_on_sale: (25000000 * 10**6).toString(),
            grace_period: (0),
            platform_fee: (0).toString(),
            decimals: (6),
            start: (current_block_timestamp),
            cliff: (current_block_timestamp * 0),
            duration: (24 * 60 * 60),
            initial_unlock_percent: (5000),
            linear_vesting_count: (5),
        }
    };

    const vestingAddress = await deployContract(
        "./artifacts/vesting.wasm", 
        vestingProperties, 
        client, 
        ownerAddress
    );

    return vestingAddress;
}