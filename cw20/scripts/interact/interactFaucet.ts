import { toBinary } from "@cosmjs/cosmwasm-stargate";

const addTokens = async (client: any, ownerAddress: any) => {
    await client.execute(ownerAddress, "", {
        send: {
        contract: "",
        amount: (10 * 10 ** 6).toString(), 
        msg: toBinary({ add_tokens: {} })
        }
    }, "auto");

    await client.execute(ownerAddress, "", { add_native_tokens: {} }, "auto", undefined, [{
        amount: (200000 * 10 ** 6).toString(), 
        denom: ""
    }]);
};

const updateConfig = async (client: any, ownerAddress: any) => {
    await client.execute(ownerAddress, "", {
        update_config: {
        claim_interval: (1 * 24 * 60 * 60).toString(),
        native_coin: {
            denom: "",
            amount: (1 * 10**6).toString()
        },
        tokens: [
            {
            address: "",
            amount: (250 * 10**6).toString()
            }
        ],
        }
    }, "auto");
};

const getTokenBalance = async (client: any, ownerAddress: any) => {
    const response = await client.queryContractSmart("", {
        get_token_balance: { address: "" }
    }, "auto");

    console.log("Token Balance: ", (Math.floor(response.balance / 10**6)).toLocaleString());
}

const getNativeTokenBalance = async (client: any, ownerAddress: any) => {
    const response = await client.queryContractSmart("", {
        get_native_balance: {}
    }, "auto");

    console.log("Native Balance: ", (Math.floor(response.balance / 10**6)).toLocaleString());
}

export const interactFaucetContract = async (client: any, ownerAddress: any) => {
    await addTokens(client, ownerAddress);
    // await updateConfig(client, ownerAddress);
    await getTokenBalance(client, ownerAddress);
    await getNativeTokenBalance(client, ownerAddress);
}