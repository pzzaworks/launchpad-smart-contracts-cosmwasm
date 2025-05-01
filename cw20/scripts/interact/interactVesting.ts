import fs from 'fs';

interface WhitelistUser {
    address: string;
    totalPayment: string;
    totalSaleToken: string;
}

const loadWhitelistData = (filePath: string): WhitelistUser[] => {
    const rawData = fs.readFileSync(filePath, 'utf8');
    return JSON.parse(rawData);
};

const processWhitelistData = (users: WhitelistUser[]) => {
    const wallets = users.map(user => user.address);
    const paymentAmounts = users.map(user => {
        const amount = BigInt(user.totalPayment);
        return amount.toString();
    });
    const tokenAmounts = users.map(user => {
        const amount = BigInt(user.totalSaleToken);
        return amount.toString();
    });
  
    const totalTokenAmount = users.reduce((sum, user) => {
        return sum + BigInt(user.totalSaleToken);
    }, BigInt(0));

    console.log("Total Token Amount:", totalTokenAmount.toString());

    return { wallets, paymentAmounts, tokenAmounts };
};

const setInitialWhitelist = async (client: any, ownerAddress: string, users: WhitelistUser[]) => {
    const initialUsers = users.slice(0, 10);
    const { wallets, paymentAmounts, tokenAmounts } = processWhitelistData(initialUsers);

    console.log("Setting initial whitelist with first 10 users");
    await client.execute(
        ownerAddress,
        "",
        {
            set_whitelist: {
                tag_id: "tag1",
                wallets: wallets,
                payment_amounts: paymentAmounts,
                token: "",
                token_amounts: tokenAmounts,
                refund_fee: "500"
            }
        },
        "auto",
        undefined,
        [],
        { gas: "30000000" }
    );
    console.log("Initial whitelist set successfully");
};

const addWhitelistBatch = async (client: any, ownerAddress: string, users: WhitelistUser[]) => {
    const initialUsers = users.slice(10);
    const batchSize = 50;
    const totalBatches = Math.ceil(initialUsers.length / batchSize);

    for (let i = 0; i < initialUsers.length; i += batchSize) {
        const batchUsers = initialUsers.slice(i, i + batchSize);
        const { wallets, paymentAmounts, tokenAmounts } = processWhitelistData(batchUsers);

        const currentBatch = Math.floor(i / batchSize) + 1;

        console.log(`Setting whitelist batch ${currentBatch} of ${totalBatches} (Users ${i + 1} to ${Math.min(i + batchSize, initialUsers.length)})`);

        let success = false;
        while (!success) {
            try {
                await client.execute(
                    ownerAddress,
                    "",
                    {
                        add_to_whitelist: {
                            tag_id: "tag1",
                            wallets: wallets,
                            payment_amounts: paymentAmounts,
                            token: "",
                            token_amounts: tokenAmounts,
                            refund_fee: "500"
                        }
                    },
                    "auto",
                    undefined,
                    [],
                    { gas: "30000000" }
                );

                console.log(`Successfully set whitelist batch ${currentBatch} of ${totalBatches}`);
                success = true;
            } catch (error) {
                console.error(`Error in batch ${currentBatch} of ${totalBatches}:`, error);
                console.log(`Retrying batch ${currentBatch}...`);
                await new Promise(resolve => setTimeout(resolve, 1000)); 
            }
        }

        await new Promise(resolve => setTimeout(resolve, 100));
    }
};

const sendTokens = async (client: any, ownerAddress: any) => {
    await client.execute(
        ownerAddress,
        "",
        {
            transfer: {
                recipient: "",
                amount: (25000000 * 10**6).toString()
            }
        },
        "auto"
    );
};

const updateToken = async (client: any, ownerAddress: any) => {
    await client.execute(ownerAddress, "", {
        update_token: { new_token: "" }
    }, "auto");
};

const setVestingStart = async (client: any, ownerAddress: any) => {
    await client.execute(ownerAddress, "", {
        set_vesting_start: { new_start: 1724714574 }
    }, "auto");
};

const getPaymentToken = async (client: any, ownerAddress: any) => {
    const response = await client.queryContractSmart("", {
        get_payment_token: { tag_id: "tag1" }
    }, "auto");

    console.log(response);
}

const getUserInfo = async (client: any, ownerAddress: any) => {
    const response = await client.queryContractSmart(
        "",
        {
            get_user_info: { 
                tag_id: "tag1", 
                wallet: "" 
            }
        },
        "auto"
    );

    console.log(response);
};

const tokenBalance = async (client: any, ownerAddress: any) => {
    const response = await client.queryContractSmart("", {
        balance: { address: ""}
    }, "auto");

    console.log(response);
}

export const interactVestingContract = async (client: any, ownerAddress: any) => {
    const whitelistData = loadWhitelistData('scripts/interact/build/whitelist.json');
    
    // await setInitialWhitelist(client, ownerAddress, whitelistData);
    // await addWhitelistBatch(client, ownerAddress, whitelistData);
    
    await sendTokens(client, ownerAddress); 
    // await tokenBalance(client, ownerAddress);
    // await updateToken(client, ownerAddress);
    // await setVestingStart(client, ownerAddress);
    // await getPaymentToken(client, ownerAddress);
    // await getUserInfo(client, ownerAddress);
};