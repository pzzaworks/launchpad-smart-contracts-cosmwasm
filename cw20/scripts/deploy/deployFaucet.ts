import { deployContract } from './deployContract';

export const deployFaucetContract = async (client: any, ownerAddress: any) => {
    const faucetProperties = {
        label: "Faucet Contract",
        properties: {
            owner: ownerAddress,
            tokens: [
                {
                    address: "",
                    amount: (250 * 10**6).toString()
                }
            ],
            native_coin: {
                denom: "",
                amount: (1 * 10**3).toString()
            },
            claim_interval: (24 * 60 * 60).toString()
        }
    };

    const faucetAddress = await deployContract(
        "./artifacts/faucet.wasm", 
        faucetProperties, 
        client, 
        ownerAddress
    );

    return faucetAddress;
}