const withdraw = async (client: any, ownerAddress: any) => {
    await client.execute(ownerAddress, "", {
        withdraw: {}
    }, "auto");
};



export const interactSaleContract = async (client: any, ownerAddress: any) => {
    await withdraw(client, ownerAddress);
}