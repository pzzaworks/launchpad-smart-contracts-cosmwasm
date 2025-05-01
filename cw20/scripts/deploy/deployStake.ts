import { deployContract } from './deployContract';

export const deployStakeContract = async (client: any, ownerAddress: any) => {
    const stakeProperties = {
        label: "Stake Contract",
        properties: {
            owner: ownerAddress,
            token_address: "",
            stake_paused: false,
            unstake_paused: false,
            emergency_unstake_paused: true,
            interest_rate: (500).toString(),
            lock_duration: (1 * 60 * 60).toString(),
            lock_duration_multiplier: (500000).toString(),
            emergency_unstake_fee_percentage: (500).toString(),
            fee_address: ownerAddress
        }
    };

    const stakeAddress = await deployContract(
        "./artifacts/stake.wasm", 
        stakeProperties, 
        client, 
        ownerAddress
    );

    return stakeAddress;
}