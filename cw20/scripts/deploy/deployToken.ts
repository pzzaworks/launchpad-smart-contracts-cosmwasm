import fs from 'fs';

export const deployTokenContract = async (client: any, ownerAddress: any) => {
    console.log("Deploying Contract...");

    const contractCode = fs.readFileSync("./artifacts/token.wasm");
    const uploadReceipt = await client.upload(ownerAddress, contractCode, 'auto');

    console.log("Contract code deployed by: ", ownerAddress);
    console.log("Upload receipt tx hash: ", uploadReceipt.transactionHash);
    console.log("Instantiating Contract...");

    try {
        const instantiateReceipt = await client.instantiate(
            ownerAddress,
            uploadReceipt.codeId,
            {
              name: "Test",
              symbol: "TEST",
              decimals: 6,
              initial_balances: [
                {
                  address: ownerAddress,
                  amount: (1000000000*10**6).toString(),
                },
              ],
            },
            "Token Contract",
            'auto'
        );
        console.log("Contract deployed successfully at address:", instantiateReceipt.contractAddress);
        return instantiateReceipt.contractAddress
    } catch (error) {
        console.error("Error while deploying Contract:", error);
    }

    return null;
}