interface FilteredTransaction {
    txHash: string;
    sender: string;
    paymentAmount: string;
    saleTokenAmount: string;
    timestamp: string;
}

async function getAllTransactions(client: any) {
    console.log("Fetching transactions for the last 3 days...");
    const contractAddress: string = "";
    let contractTransactions: Map<string, any> = new Map();
    const averageBlockTime = 6;
    const threeDaysInSeconds = 2 * 22 * 60 * 60;
    const maxRetries = 3;
    const emptyIntervalLimit = 3;

    const timeIntervals = [
        24 * 60 * 60, 12 * 60 * 60, 6 * 60 * 60, 3 * 60 * 60, 1 * 60 * 60,
        30 * 60, 15 * 60, 5 * 60, 1 * 60, 30,
    ];

    try {
        const latestBlock = await client.getBlock();
        const latestBlockHeight = latestBlock.header.height;
        
        const blocksInThreeDays = Math.floor(threeDaysInSeconds / averageBlockTime);
        const startBlockHeight = Math.max(1, latestBlockHeight - blocksInThreeDays);

        console.log(`Fetching transactions from block ${startBlockHeight} to ${latestBlockHeight}`);

        let emptyIntervalCount = 0;
        let shouldContinueSearch = true;

        async function fetchBlockRange(start: number, end: number, intervalIndex = 0): Promise<boolean> {
            if (intervalIndex >= timeIntervals.length || !shouldContinueSearch) {
                console.log(`Reached minimum interval or search stopped for blocks ${start} to ${end}. Skipping.`);
                return false;
            }

            const interval = timeIntervals[intervalIndex];
            const blocksInInterval = Math.floor(interval / averageBlockTime);
            
            for (let currentBlock = start; currentBlock <= end; currentBlock += blocksInInterval) {
                if (!shouldContinueSearch) break;
                
                const endBlock = Math.min(currentBlock + blocksInInterval - 1, end);
                console.log(`Attempting to fetch transactions for blocks ${currentBlock} to ${endBlock}`);
                
                let success = false;
                for (let attempt = 0; attempt < maxRetries; attempt++) {
                    try {
                        const result = await client.searchTx(
                            `wasm._contract_address='${contractAddress}' AND tx.height>=${currentBlock} AND tx.height<=${endBlock}`,
                            { order_by: "ORDER_BY_ASC" }
                        );

                        if (result && result.length > 0) {
                            let newTxCount = 0;
                            for (const tx of result) {
                                if (!contractTransactions.has(tx.hash)) {
                                    contractTransactions.set(tx.hash, tx);
                                    newTxCount++;
                                }
                            }
                            
                            console.log(`Fetched ${result.length} transactions, ${newTxCount} new from blocks ${currentBlock} to ${endBlock}`);
                            console.log(`Total unique transactions fetched so far: ${contractTransactions.size}`);
                            success = true;
                            emptyIntervalCount = 0;
                            break;
                        } else {
                            console.log(`No transactions found from blocks ${currentBlock} to ${endBlock} (Attempt ${attempt + 1}/${maxRetries})`);
                        }
                    } catch (error: any) {
                        console.log(`Error fetching transactions for blocks ${currentBlock} to ${endBlock}. Attempt ${attempt + 1}/${maxRetries}`);
                    }
                }

                if (!success) {
                    console.log(`Failed to fetch transactions for blocks ${currentBlock} to ${endBlock} after ${maxRetries} attempts. Splitting interval.`);
                    if (interval === 1 * 60 * 60) {
                        console.log(`No transactions found in 1-hour interval. Stopping search after ${emptyIntervalLimit} consecutive intervals.`);
                        emptyIntervalCount++;
                        if (emptyIntervalCount >= emptyIntervalLimit) {
                            console.log(`No transactions found in ${emptyIntervalLimit} consecutive 6-hour intervals. Stopping search.`);
                            shouldContinueSearch = false;
                            break;
                        }
                    }
                    await fetchBlockRange(currentBlock, endBlock, intervalIndex + 1);
                }
            }

            return true;
        }

        await fetchBlockRange(startBlockHeight, latestBlockHeight);

        console.log(`Final total unique transactions fetched: ${contractTransactions.size}`);
        return Array.from(contractTransactions.values());
    } catch (error) {
        console.error(`Error in overall transaction fetching process:`, error);
        return Array.from(contractTransactions.values());
    }
}

function safeStringify(obj: any) {
    return JSON.stringify(obj, (key, value) =>
        typeof value === 'bigint'
            ? value.toString()
            : value
    );
}

async function saveAllTransactions(client: any) {
    const allTxs = await getAllTransactions(client);
    console.log(`Total transactions: ${allTxs.length}`);
    
    const jsonContent = safeStringify(allTxs);
    require('fs').mkdirSync('scripts/interact/build/sale4', { recursive: true });
    require('fs').writeFileSync('scripts/interact/build/sale4/all_transactions.json', jsonContent);
    console.log('All transactions have been saved to all_transactions.json');
}

function filterTransactionsFromFile() {
    const fs = require('fs');
    const path = require('path');

    const inputFilePath = path.join('scripts', 'interact', 'build', 'sale4', 'all_transactions.json');
    const outputFilePath = path.join('scripts', 'interact', 'build', 'sale4', 'unique_addresses_with_totals.json');

    const fileContent = fs.readFileSync(inputFilePath, 'utf-8');
    const allTxs = JSON.parse(fileContent);

    const uniqueAddresses: { [key: string]: { totalPayment: string, totalSaleToken: string } } = {};
    
    const filteredTxs = allTxs.filter((tx: any) => {
        const wasmEvents = tx.events?.find((event: any) => event.type === 'wasm');
        if (!wasmEvents) return false;

        return wasmEvents.attributes.some((attr: any) => 
            attr.key === 'method' && attr.value === 'join_fcfs_round'
        );
    }).map((tx: any) => {
        const wasmEvents = tx.events.find((event: any) => event.type === 'wasm');
        const getAttributeValue = (key: string) => {
            const attr = wasmEvents.attributes.find((attr: any) => attr.key === key);
            return attr ? attr.value : '';
        };

        const transferEvents = tx.events.find((event: any) => event.type === 'transfer');
        const getTransferValue = (key: string) => {
            const attr = transferEvents.attributes.find((attr: any) => attr.key === key);
            return attr ? attr.value : '';
        };

        let timestamp = tx.timestamp;
        if (typeof timestamp === 'string') {
            timestamp = timestamp.replace(' ', 'T') + 'Z'; 
        } else if (typeof timestamp === 'number') {
            timestamp = new Date(timestamp * 1000).toISOString();
        } else {
            timestamp = new Date().toISOString();
        }

        const sender = getAttributeValue('user');
        const paymentAmount = (getTransferValue('amount') || '').split('ibc/')[0] || '';
        const saleTokenAmount = getAttributeValue('sale_token_amount');

        if (uniqueAddresses[sender]) {
            uniqueAddresses[sender].totalPayment = (BigInt(uniqueAddresses[sender].totalPayment) + BigInt(paymentAmount)).toString();
            uniqueAddresses[sender].totalSaleToken = (BigInt(uniqueAddresses[sender].totalSaleToken) + BigInt(saleTokenAmount)).toString();
        } else {
            uniqueAddresses[sender] = {
                totalPayment: paymentAmount,
                totalSaleToken: saleTokenAmount
            };
        }

        return {
            txHash: tx.hash || '',
            sender: sender,
            paymentAmount: paymentAmount,
            saleTokenAmount: saleTokenAmount,
            timestamp: timestamp
        };
    });

    const uniqueAddressesArray = Object.entries(uniqueAddresses).map(([address, data]) => ({
        address,
        totalPayment: data.totalPayment,
        totalSaleToken: data.totalSaleToken
    }));

    fs.mkdirSync(path.dirname(outputFilePath), { recursive: true });
    fs.writeFileSync(outputFilePath, JSON.stringify(uniqueAddressesArray, null, 2));
    console.log(`Number of unique addresses: ${uniqueAddressesArray.length}`);

    return filteredTxs;
}

function calculateTotalPaymentAmount(transactions: FilteredTransaction[]): number {
    return transactions.reduce((total, tx) => {
        const amount = parseFloat(tx.paymentAmount);
        return isNaN(amount) ? total : total + amount;
    }, 0);
}

export const getAllTx = async (client: any, ownerAddress: any) => {
    // await saveAllTransactions(client);
    const filteredTxs = filterTransactionsFromFile();
    console.log(`Filtered transactions: ${filteredTxs.length}`);
    
    if (filteredTxs.length > 0) {
        console.log("First filtered transaction:", JSON.stringify(filteredTxs[0], null, 2));
        const totalPaymentAmount = calculateTotalPaymentAmount(filteredTxs);
        console.log(`Total payment amount: ${totalPaymentAmount}`);
    } else {
        console.log("No transactions matched the filter criteria.");
    }
    
    const jsonContent = JSON.stringify(filteredTxs, null, 2);
    require('fs').mkdirSync('scripts/interact/build/sale4', { recursive: true });
    require('fs').writeFileSync('scripts/interact/build/sale4/filtered_transactions.json', jsonContent);
    console.log('Filtered transactions have been saved to filtered_transactions.json');
    console.log('Unique addresses have been saved to unique_addresses.json');
}