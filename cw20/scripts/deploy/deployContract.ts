import fs from 'fs';

export const deployContract = async (filePath: string, contractProperties: any, client: any, ownerAddress: any) => {
    console.log("Deploying Contract...");

    const contractCode = fs.readFileSync(filePath);
    const uploadReceipt = await client.upload(ownerAddress, contractCode, 'auto');

    console.log("Contract code deployed by: ", ownerAddress);
    console.log("Upload receipt tx hash: ", uploadReceipt.transactionHash);
    console.log("Instantiating Contract...");

    try {
        const instantiateReceipt = await client.instantiate(
            ownerAddress,
            uploadReceipt.codeId,
            contractProperties.properties,
            contractProperties.label,
            'auto'
        );
        console.log("Contract deployed successfully at address:", instantiateReceipt.contractAddress);
        return instantiateReceipt.contractAddress
    } catch (error) {
        console.error("Error while deploying Contract:", error);
    }

    return null;
}