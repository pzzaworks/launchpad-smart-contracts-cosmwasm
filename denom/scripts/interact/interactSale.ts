const register = async (client: any, ownerAddress: any) => {
    await client.execute(ownerAddress, "", {
        register: {}
    }, "auto");
}

const joinStakerRound = async (client: any, ownerAddress: any) => {
    await client.execute(ownerAddress, "", {
          join_staker_round: {
            proof: []
          }
        },
        "auto",
        "", 
        [{ denom: "", amount: "1000" }]
      );
}

const withdraw = async (client: any, ownerAddress: any) => {
    await client.execute(ownerAddress, "", {
        withdraw: {}
    }, "auto");
};

const updateConfig = async (client: any, ownerAddress: any) => {
    const current_block_timestamp = Number(Math.floor(new Date((await client.getBlock()).header.time).getTime() / 1000));

    await client.execute(ownerAddress, "", {
        update_config: {
            stake_controller: "",
            payment_denom: "",
            sale_token_decimals: (6).toString(),
            sale_token_price: (1*10**3).toString(),
            min_allocation: (1*10**3).toString(),
            total_allocation: (315000*10**6).toString(),
            fcfs_allocation: (0).toString(), // 0 for unlimited allocation
            status: {
                register_paused: false,
                staker_paused: false,
                fcfs_paused: false,
            },
            dates: {
                register_start: (1724321991).toString(),
                register_end: (1724321991).toString(),
                staker_start: (1724321991).toString(),
                staker_end: (1724321991).toString(),
                fcfs_start: (1724321991).toString(),
                fcfs_end: (1724761980).toString(),
            },
            whitelist_properties: {
                whitelist_merkle_root: ("").toString(),
                whitelisted_user_count: (0).toString(),
                whitelisted_user_allocation: (100*10**6).toString(),
            },
        }
    }, "auto");
};

async function getAllUserInfo(client: any, contractAddress: string) {
    let allUsers: any = [];
    let startAfter: string | null = null;
    const limit = 100; 
    let totalFetched = 0;

    while (true) {
        let retry = true;
        while (retry) {
            try {
                console.log(`Fetching users from ${startAfter}...`);

                const msg = {
                    get_all_user_info_at_height: {
                        start_after: startAfter,
                        limit: limit,
                        height: null  
                    }
                };

                const response: any = await client.queryContractSmart(contractAddress, msg);

                allUsers = allUsers.concat(response.user_infos);
                totalFetched += response.user_infos.length;

                if (response.user_infos.length < limit) {
                    console.log(`Fetched ${response.user_infos.length} users from ${startAfter}...`);
                    console.log(`Total users fetched: ${totalFetched}`);
                    return allUsers;
                }

                startAfter = response.user_infos[response.user_infos.length - 1].address;

                console.log(`Fetched ${response.user_infos.length} users from ${startAfter}...`);
                console.log(`Total users fetched so far: ${totalFetched}`);

                retry = false; 
            } catch (error) {
                console.error(`Error occurred while fetching users: ${error}`);
                console.log('Retrying...');
            }
        }
    }
}

async function fetchAndProcessAllUsers(client: any) {
    try {
        const allUsers = await getAllUserInfo(client, "");
        console.log(`Total users fetched: ${allUsers.length}`);

        const totalPaymentAmount = allUsers.reduce((sum: any, user: any) => sum + parseFloat(user.total_payment_amount), 0);
        const totalSaleTokenAmount = allUsers.reduce((sum: any, user: any) => sum + parseFloat(user.total_sale_token_amount), 0);

        const totals = {
            totalPaymentAmount,
            totalSaleTokenAmount
        };

        require('fs').mkdirSync('scripts/interact/build/snake/sale1', { recursive: true });
        
        require('fs').writeFileSync('scripts/interact/build/snake/sale1/total_amounts.json', JSON.stringify(totals, null, 2));
        console.log('All user information has been saved to total_amounts.json');
        
        require('fs').writeFileSync('scripts/interact/build/snake/sale1/all_users_info.json', JSON.stringify(allUsers, null, 2));
        console.log('All user information has been saved to all_users_info.json');

    } catch (error) {
        console.error("Error fetching user info:", error);
    }
}

async function processAndFilterSaleData() {
    const inputFilePath = require('path').join('scripts', 'interact', 'build', 'snake/sale1', 'all_users_info.json');
    const outputFilePath = require('path').join('scripts', 'interact', 'build', 'snake/sale1', 'unique_addresses_with_totals.json');

    const fileContent = require('fs').readFileSync(inputFilePath, 'utf-8');
    const allEntries = JSON.parse(fileContent);

    const uniqueEntries: Array<{ address: string, totalPayment: string, totalSaleToken: string }> = [];

    const processedAddresses: { [key: string]: number } = {};

    allEntries.forEach((entry: any) => {
        const address = entry.address;
        const paymentAmount = entry.total_payment_amount || '0';
        const saleTokenAmount = entry.total_sale_token_amount || '0';

        if (BigInt(paymentAmount) === BigInt(0)) {
            return; 
        }

        if (address in processedAddresses) {
            const index = processedAddresses[address];
            uniqueEntries[index].totalPayment = (BigInt(uniqueEntries[index].totalPayment) + BigInt(paymentAmount)).toString();
            uniqueEntries[index].totalSaleToken = (BigInt(uniqueEntries[index].totalSaleToken) + BigInt(saleTokenAmount)).toString();
        } else {
            processedAddresses[address] = uniqueEntries.length;
            uniqueEntries.push({
                address: address,
                totalPayment: paymentAmount,
                totalSaleToken: saleTokenAmount
            });
        }
    });

    const filteredAndUniqueEntries = Object.values(uniqueEntries);

    require('fs').writeFileSync(outputFilePath, JSON.stringify(filteredAndUniqueEntries, null, 2));

    console.log(`Processed ${allEntries.length} entries`);
    console.log(`Number of unique addresses with non-zero payment: ${filteredAndUniqueEntries.length}`);

    const totalPayment = filteredAndUniqueEntries.reduce((sum, entry) => sum + BigInt(entry.totalPayment), BigInt(0));
    const totalSaleToken = filteredAndUniqueEntries.reduce((sum, entry) => sum + BigInt(entry.totalSaleToken), BigInt(0));

    console.log(`Total payment amount: ${totalPayment.toString()}`);
    console.log(`Total sale token amount: ${totalSaleToken.toString()}`);

    return filteredAndUniqueEntries;
}

async function mergeUniqueAddresses() {
    const inputPaths = [
        'scripts/interact/build/sale1/unique_addresses_with_totals.json',
        'scripts/interact/build/sale2/unique_addresses_with_totals.json',
        'scripts/interact/build/snake/sale1/unique_addresses_with_totals.json',
    ];

    const outputPath = 'scripts/interact/build/sale4/all_users_info.json';

    const mergedAddresses: { [key: string]: { totalPayment: bigint, totalSaleToken: bigint } } = {};

    let totalPaymentSum = BigInt(0);
    let totalSaleTokenSum = BigInt(0);

    inputPaths.forEach(inputPath => {
        const fileContent = require('fs').readFileSync(inputPath, 'utf-8');
        const data = JSON.parse(fileContent);

        data.forEach((entry: { address: string, totalPayment: string, totalSaleToken: string }) => {
            if (mergedAddresses[entry.address]) {
                mergedAddresses[entry.address].totalPayment += BigInt(entry.totalPayment);
                mergedAddresses[entry.address].totalSaleToken += BigInt(entry.totalSaleToken);
            } else {
                mergedAddresses[entry.address] = {
                    totalPayment: BigInt(entry.totalPayment),
                    totalSaleToken: BigInt(entry.totalSaleToken)
                };
            }
            totalPaymentSum += BigInt(entry.totalPayment);
            totalSaleTokenSum += BigInt(entry.totalSaleToken);
        });
    });

    const resultArray = Object.entries(mergedAddresses).map(([address, data]) => ({
        address,
        totalPayment: data.totalPayment.toString(),
        totalSaleToken: data.totalSaleToken.toString()
    }));

    require('fs').mkdirSync(require('path').dirname(outputPath), { recursive: true });
    require('fs').writeFileSync(outputPath, JSON.stringify(resultArray, null, 2));

    console.log(`Total unique addresses: ${resultArray.length}`);
    console.log(`Total Payment Amount: ${totalPaymentSum.toString()}`);
    console.log(`Total Sale Token Amount: ${totalSaleTokenSum.toString()}`);
    console.log(`Result file created: ${outputPath}`);

    return resultArray;
}

const getStatistics = async (client: any, ownerAddress: any) => {
    const response = await client.queryContractSmart("", {
        get_statistics: {}
    }, "auto");

    console.log(response);
}

const getBalance = async (client: any, ownerAddress: any) => {
    const response = await client.queryContractSmart("", {
        get_balance: {}
    }, "auto");

    console.log(response);
}

const getConfig = async (client: any, ownerAddress: any) => {
    const response = await client.queryContractSmart("", {
        get_config: {}
    }, "auto");

    console.log(response);
}

const getUserInfo = async (client: any, ownerAddress: any) => {
    const response = await client.queryContractSmart("", {
        get_user_info_at_height: { address: "", height: null }
    }, "auto");

    console.log(response);
}

const getUserStakerAllocation = async (client: any, ownerAddress: any) => {
    const response = await client.queryContractSmart("", {
        get_user_staker_allocation: { address: "" }
    }, "auto");

    console.log(response);
}

export const interactSaleContract = async (client: any, ownerAddress: any) => {
    await register(client, ownerAddress);
    // await joinStakerRound(client, ownerAddress);
    // await withdraw(client, ownerAddress); 
    // await updateConfig(client, ownerAddress);
    // await getUserInfo(client, ownerAddress);
    // await getConfig(client, ownerAddress);
    // await getStatistics(client, ownerAddress);
    // await getUserStakerAllocation(client, ownerAddress);
    // await getBalance(client, ownerAddress);
    // await fetchAndProcessAllUsers(client);
    // await processAndFilterSaleData();
    // await mergeUniqueAddresses();
}