import { deployContract } from './deployContract';

export const deployStakeControllerContract = async (client: any, ownerAddress: any) => {
    const stakeControllerProperties = {
        label: "Stake Controller Contract",
        properties: {
            owner: ownerAddress,
            token_address: "",
            stake_contracts: [
                ""
            ],
            stake_contract_multipliers: [
                (10000).toString()
            ],
            tier_thresholds: [
                (100*10**6).toString(),
                (200*10**6).toString(),
                (500*10**6).toString(),
                (1000*10**6).toString()
            ]
        }
    };

    const stakeControllerAddress = await deployContract(
      "./artifacts/stake_controller.wasm", 
      stakeControllerProperties, 
      client, 
      ownerAddress
    );

    return stakeControllerAddress;
}