const addTokens = async (client: any, ownerAddress: any) => {
    await client.execute(ownerAddress, "", {
        send: {
            amount: String(2000000*(10**6)),
            contract: "",
            msg: btoa(JSON.stringify({ add_tokens: {} })),
        }
    }, "auto");
};

export const interactStakeContract = async (client: any, ownerAddress: any) => {
    await addTokens(client, ownerAddress);
}