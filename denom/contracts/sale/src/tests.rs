use std::ops::Mul;
use crate::msg::{
    ExecuteMsg, GetAllUserInfoAtHeightResponse, GetStatisticsResponse, GetUserInfoAtHeightResponse, GetUserTierResponse, InstantiateMsg, QueryMsg
};
use crate::state::{Config, Dates, Statistics, Status, UserInfo, WhitelistProperties};
use crate::ContractError;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{coin, to_json_binary, Addr, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult, Timestamp, Uint128, Uint64};
use cw_multi_test::{next_block, App, AppResponse, Contract, ContractWrapper, Executor};
use anyhow::Result as AnyResult;
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static USER_STAKES: RefCell<HashMap<String, Uint128>> = RefCell::new(HashMap::new());
}

const ADDR1: &str = "addr0001";
const OWNER: &str = "owner";
const PAYMENT_DENOM: &str = "uusd";

fn contract_sale() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

fn contract_stake_controller() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mock_stake_controller_execute,
        mock_stake_controller_instantiate,
        mock_stake_controller_query,
    );
    Box::new(contract)
}

#[cw_serde]
pub struct GetUserStakeResponse {
    pub stake: Uint128,
}

#[cw_serde]
enum MockQueryMsg {
    GetUserStake { address: String },
    GetUserTierIndex { address: String },
}

fn mock_stake_controller_query(
    _deps: Deps,
    _env: Env,
    msg: MockQueryMsg,
) -> StdResult<Binary> {
    match msg {
        MockQueryMsg::GetUserTierIndex { address } => {
            let stake = USER_STAKES.with(|stakes| {
                stakes.borrow().get(&address).cloned().unwrap_or_default()
            });
            let response = GetUserTierResponse { tier: Uint64::new(1), total_staked: stake };
            to_json_binary(&response)
        },
        MockQueryMsg::GetUserStake { address } => {
            let stake = USER_STAKES.with(|stakes| {
                stakes.borrow().get(&address).cloned().unwrap_or_default()
            });
            let response = GetUserStakeResponse { stake };
            to_json_binary(&response)
        },
    }
}

fn mock_stake_controller_instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
) -> Result<Response, ContractError> {
    Ok(Response::new().add_attribute("method", "instantiate"))
}

fn mock_stake_controller_execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
) -> Result<Response, ContractError> {
    Ok(Response::new().add_attribute("method", "execute"))
}

fn instantiate_stake_controller(app: &mut App) -> Addr {
    let stake_controller_code_id = app.store_code(contract_stake_controller());
    let msg = Empty {};

    app.instantiate_contract(
        stake_controller_code_id,
        Addr::unchecked(ADDR1),
        &msg,
        &[],
        "stake_controller",
        Some("admin".to_string()),
    ).unwrap()
}

fn mock_app() -> App {
    App::default()
}

fn instantiate_sale(app: &mut App, stake_controller_address: Addr) -> Addr {
    let sale_code_id = app.store_code(contract_sale());
    let msg = InstantiateMsg {
        owner: Some(OWNER.to_string()),
        stake_controller: stake_controller_address.to_string(),
        payment_denom: PAYMENT_DENOM.to_string(),
        sale_token_decimals: Uint64::new(6),
        sale_token_price: Uint128::new(1u128.mul(10u128.pow(5))),
        min_allocation: Uint128::new(10u128.mul(10u128.pow(6))),
        total_allocation: Uint128::new(100000000u128.mul(10u128.pow(6))),
        fcfs_allocation: Uint128::new(0),
        status: Status {
            register_paused: false,
            staker_paused: false,
            fcfs_paused: false,
        },
        dates: Dates {
            register_start: Uint64::from(mock_env().block.time.seconds()),
            register_end: Uint64::from(mock_env().block.time.plus_seconds(10000).seconds()),
            staker_start: Uint64::from(mock_env().block.time.plus_seconds(20000).seconds()),
            staker_end: Uint64::from(mock_env().block.time.plus_seconds(30000).seconds()),
            fcfs_start: Uint64::from(mock_env().block.time.plus_seconds(40000).seconds()),
            fcfs_end: Uint64::from(mock_env().block.time.plus_seconds(50000).seconds()),
        },
        whitelist_properties: WhitelistProperties {
            whitelist_merkle_root: String::new(),
            whitelisted_user_count: Uint128::new(100),
            whitelisted_user_allocation: Uint128::new(100u128.mul(10u128.pow(6))),
        },
    };
    app.instantiate_contract(
        sale_code_id,
        Addr::unchecked(ADDR1),
        &msg,
        &[],
        "sale",
        Some("admin".to_string()),
    ).unwrap()
}

fn setup_test_case(app: &mut App) -> Addr {
    app.sudo(cw_multi_test::SudoMsg::Bank(
        cw_multi_test::BankSudo::Mint {
            to_address: ADDR1.to_string(),
            amount: vec![coin(1_000_000_000, PAYMENT_DENOM)],
        },
    ))
    .unwrap();

    let stake_controller_addr = instantiate_stake_controller(app);
    app.update_block(next_block);
    
    let sale_addr = instantiate_sale(app, stake_controller_addr.clone());
    app.update_block(next_block);
    
    sale_addr
}

fn register_user(app: &mut App, sale_addr: &Addr, info: MessageInfo, proof: Option<Vec<String>>) -> AnyResult<AppResponse> {
    let msg = ExecuteMsg::Register {
        proof: Some(proof.unwrap_or_default()),
    };
    app.execute_contract(info.sender, sale_addr.clone(), &msg, &[])
}

fn join_staker_round(
    app: &mut App,
    sale_addr: &Addr,
    info: MessageInfo,
    amount: Uint128,
    proof: Option<Vec<String>>,
) -> AnyResult<AppResponse> {
    let msg = ExecuteMsg::JoinStakerRound {
        proof: proof,
    };
    app.execute_contract(info.sender, sale_addr.clone(), &msg, &[coin(amount.u128(), PAYMENT_DENOM)])
}

fn join_fcfs_round(
    app: &mut App,
    sale_addr: &Addr,
    info: MessageInfo,
    amount: Uint128,
) -> AnyResult<AppResponse> {
    let msg = ExecuteMsg::JoinFcfsRound {};
    app.execute_contract(info.sender, sale_addr.clone(), &msg, &[coin(amount.u128(), PAYMENT_DENOM)])
}

fn query_user_info<T: Into<String>>(app: &App, contract_addr: T, address: String, height: Option<u64>) -> UserInfo {
    let msg = QueryMsg::GetUserInfoAtHeight { address, height };
    let result: GetUserInfoAtHeightResponse = app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
    result.user_info
}

#[test]
fn test_register_user() {
    let mut app = mock_app();
    let sale_addr = setup_test_case(&mut app);

    USER_STAKES.with(|stakes| {
        stakes.borrow_mut().insert(ADDR1.to_string(), Uint128::new(100u128.mul(10u128.pow(6))));
    });

    let info = mock_info(ADDR1, &[]);
    register_user(&mut app, &sale_addr, info.clone(), None).unwrap();
    app.update_block(next_block);

    let block_height = app.block_info().height;
    let user_info = query_user_info(&app, sale_addr, ADDR1.to_string(), Some(block_height));
    
    assert!(user_info.registered);
}

#[test]
fn test_join_staker_round() {
    let mut app = mock_app();
    let sale_addr = setup_test_case(&mut app);
    let info = mock_info(ADDR1, &[]);

    let initial_balance = app.wrap().query_balance(ADDR1, PAYMENT_DENOM).unwrap();
    let initial_config: Config = app.wrap().query_wasm_smart(&sale_addr, &QueryMsg::GetConfig {}).unwrap();
    let initial_statistics: Statistics = app.wrap().query_wasm_smart::<GetStatisticsResponse>(&sale_addr, &QueryMsg::GetStatistics { height: None }).unwrap().statistics;
    
    let stake_amount = Uint128::new(100u128.mul(10u128.pow(6)));
    USER_STAKES.with(|stakes| {
        stakes.borrow_mut().insert(ADDR1.to_string(), stake_amount);
    });

    register_user(&mut app, &sale_addr, info.clone(), None).unwrap();
    app.update_block(next_block);

    let user_info_after_register = query_user_info(&app, sale_addr.clone(), ADDR1.to_string(), None);
    assert!(user_info_after_register.registered, "User should be registered");
    
    let statistics_after_register: Statistics = app.wrap().query_wasm_smart::<GetStatisticsResponse>(&sale_addr, &QueryMsg::GetStatistics { height: None }).unwrap().statistics;    
    assert_eq!(statistics_after_register.total_registered_users, initial_statistics.total_registered_users + Uint128::new(1), "Total registered users should increase by 1");

    app.update_block(|block| {
        block.time = block.time.plus_seconds(20000);
    });
    
    let amount = Uint128::new(100u128.mul(10u128.pow(6)));
    join_staker_round(&mut app, &sale_addr, info.clone(), amount, None).unwrap();
    app.update_block(next_block);

    let user_info_after_staker = query_user_info(&app, sale_addr.clone(), ADDR1.to_string(), None);
    assert!(user_info_after_staker.joined_staker_round, "User should have joined staker round");
    assert_eq!(user_info_after_staker.total_payment_amount, amount, "User's total payment amount should match");

    let expected_sale_token_amount = (amount.u128() * 10u128.pow(initial_config.sale_token_decimals.u64() as u32)) / initial_config.sale_token_price.u128();
    let expected_sale_token_amount = Uint128::from(expected_sale_token_amount);
    assert_eq!(user_info_after_staker.total_sale_token_amount, expected_sale_token_amount, "User's total sale token amount should match expected amount");

    let balance_after = app.wrap().query_balance(ADDR1, PAYMENT_DENOM).unwrap();
    assert_eq!(balance_after.amount, initial_balance.amount - amount, "User balance should be reduced after joining staker round");

    let contract_balance = app.wrap().query_balance(sale_addr.as_str(), PAYMENT_DENOM).unwrap();
    assert_eq!(contract_balance.amount, amount, "Contract balance should match the joined amount");

    let statistics_after_staker: Statistics = app.wrap().query_wasm_smart::<GetStatisticsResponse>(&sale_addr, &QueryMsg::GetStatistics { height: None }).unwrap().statistics;    
    assert_eq!(statistics_after_staker.total_staker_round_participants, initial_statistics.total_staker_round_participants + Uint128::new(1), "Total staker round participants should increase by 1");
    assert_eq!(statistics_after_staker.total_participants, initial_statistics.total_participants + Uint128::new(1), "Total participants should increase by 1");
    assert_eq!(statistics_after_staker.total_payment_amount, initial_statistics.total_payment_amount + amount, "Total payment amount should increase by the joined amount");

    let final_config: Config = app.wrap().query_wasm_smart(&sale_addr, &QueryMsg::GetConfig {}).unwrap();
    assert_eq!(final_config, initial_config, "Config should not change after joining staker round");
}

#[test]
fn test_join_fcfs_round() {
    let mut app = mock_app();
    let sale_addr = setup_test_case(&mut app);

    let initial_balance = app.wrap().query_balance(ADDR1, PAYMENT_DENOM).unwrap();
    let initial_config: Config = app.wrap()
        .query_wasm_smart::<Config>(&sale_addr, &QueryMsg::GetConfig {})
        .unwrap();
    let initial_statistics: Statistics = app.wrap()
        .query_wasm_smart::<GetStatisticsResponse>(&sale_addr, &QueryMsg::GetStatistics { height: None })
        .unwrap()
        .statistics;

    app.update_block(|block| {
        block.time = block.time.plus_seconds(40000);
    });

    let amount = Uint128::new(100_000_000);
    
    let info = mock_info(ADDR1, &[coin(amount.u128(), PAYMENT_DENOM)]);
    join_fcfs_round(&mut app, &sale_addr, info, amount).unwrap();
    app.update_block(next_block);

    let user_info_after_first = query_user_info(&app, sale_addr.clone(), ADDR1.to_string(), None);
    assert!(user_info_after_first.joined_fcfs_round, "User should have joined FCFS round");
    assert_eq!(user_info_after_first.total_payment_amount, amount, "User's total payment amount should match");

    let expected_sale_token_amount = (amount.u128() * 10u128.pow(initial_config.sale_token_decimals.u64() as u32)) / initial_config.sale_token_price.u128();
    let expected_sale_token_amount = Uint128::from(expected_sale_token_amount);
    assert_eq!(user_info_after_first.total_sale_token_amount, expected_sale_token_amount, "User's total sale token amount should match expected amount");

    let info_second = mock_info(ADDR1, &[coin(amount.u128(), PAYMENT_DENOM)]);
    join_fcfs_round(&mut app, &sale_addr, info_second, amount).unwrap();
    app.update_block(next_block);

    let user_info_after_second = query_user_info(&app, sale_addr.clone(), ADDR1.to_string(), None);
    assert_eq!(user_info_after_second.total_payment_amount, amount * Uint128::new(2), "User's total payment amount should double");
    assert_eq!(user_info_after_second.total_sale_token_amount, expected_sale_token_amount * Uint128::new(2), "User's total sale token amount should double");

    let balance_after = app.wrap().query_balance(ADDR1, PAYMENT_DENOM).unwrap();
    assert_eq!(balance_after.amount, initial_balance.amount - (amount * Uint128::new(2)), "User balance should be reduced after joining FCFS round twice");

    let contract_balance = app.wrap().query_balance(sale_addr.as_str(), PAYMENT_DENOM).unwrap();
    assert_eq!(contract_balance.amount, amount * Uint128::new(2), "Contract balance should match the total joined amount");

    let statistics_after_fcfs: Statistics = app.wrap()
        .query_wasm_smart::<GetStatisticsResponse>(&sale_addr, &QueryMsg::GetStatistics { height: None })
        .unwrap()
        .statistics;
    assert_eq!(statistics_after_fcfs.total_fcfs_round_participants, initial_statistics.total_fcfs_round_participants + Uint128::new(1), "Total FCFS round participants should increase by 1");
    assert_eq!(statistics_after_fcfs.total_participants, initial_statistics.total_participants + Uint128::new(1), "Total participants should increase by 1");
    assert_eq!(statistics_after_fcfs.total_payment_amount, initial_statistics.total_payment_amount + (amount * Uint128::new(2)), "Total payment amount should increase by the total joined amount");

    let final_config: Config = app.wrap()
        .query_wasm_smart::<Config>(&sale_addr, &QueryMsg::GetConfig {})
        .unwrap();
    assert_eq!(final_config, initial_config, "Config should not change after joining FCFS round");
}

#[test]
fn test_join_fcfs_round_with_high_concurrency() {
    let mut app = mock_app();
    let sale_addr = setup_test_case(&mut app);
    
    app.update_block(|block| {
        block.time = block.time.plus_seconds(40000);
    });

    let total_participants = 1000;
    let amount_per_participant = Uint128::new(10u128.mul(10u128.pow(6))); 

    let initial_config: Config = app.wrap()
        .query_wasm_smart::<Config>(&sale_addr, &QueryMsg::GetConfig {})
        .unwrap();
    let initial_statistics: Statistics = app.wrap()
        .query_wasm_smart::<GetStatisticsResponse>(&sale_addr, &QueryMsg::GetStatistics { height: None })
        .unwrap()
        .statistics;

    for i in 0..total_participants {
        let participant = format!("participant{}", i);
        app.sudo(cw_multi_test::SudoMsg::Bank(
            cw_multi_test::BankSudo::Mint {
                to_address: participant.clone(),
                amount: vec![coin(amount_per_participant.u128(), PAYMENT_DENOM)],
            },
        )).unwrap();

        let info = mock_info(&participant, &[coin(amount_per_participant.u128(), PAYMENT_DENOM)]);
        let join_result = join_fcfs_round(&mut app, &sale_addr, info, amount_per_participant);
        
        assert!(join_result.is_ok(), "Failed to join FCFS round for participant {}: {:?}", i, join_result.err());

        if i % 100 == 0 {
            app.update_block(next_block);
        }
    }
    
    app.update_block(next_block);
    let final_block_height = app.block_info().height;
    
    let mut total_participants_joined = 0;
    let mut total_amount_collected = Uint128::zero();

    for i in 0..total_participants {
        let participant = format!("participant{}", i);
        let user_info = query_user_info(&app, sale_addr.clone(), participant.clone(), Some(final_block_height));
        
        assert!(user_info.joined_fcfs_round, "Participant {} should have joined FCFS round", i);
        total_participants_joined += 1;
        total_amount_collected += user_info.total_payment_amount;
        
        let balance = app.wrap().query_balance(&participant, PAYMENT_DENOM).unwrap();
        assert_eq!(balance.amount, Uint128::zero(), "Participant {} should have spent all their balance", i);
    }

    assert_eq!(total_participants_joined, total_participants, "All participants should have joined");
    assert_eq!(total_amount_collected, amount_per_participant.mul(Uint128::new(total_participants)), "Total collected amount should match");

    let contract_balance = app.wrap().query_balance(sale_addr.as_str(), PAYMENT_DENOM).unwrap();
    assert_eq!(contract_balance.amount, total_amount_collected, "Contract balance should match total collected amount");

    let final_statistics: Statistics = app.wrap()
        .query_wasm_smart::<GetStatisticsResponse>(&sale_addr, &QueryMsg::GetStatistics { height: None })
        .unwrap()
        .statistics;

    assert_eq!(final_statistics.total_fcfs_round_participants, initial_statistics.total_fcfs_round_participants + Uint128::new(total_participants as u128), 
               "Total FCFS round participants should increase by the number of participants");
    assert_eq!(final_statistics.total_participants, initial_statistics.total_participants + Uint128::new(total_participants as u128), 
               "Total participants should increase by the number of participants");
    assert_eq!(final_statistics.total_payment_amount, initial_statistics.total_payment_amount + total_amount_collected, 
               "Total payment amount should increase by the total collected amount");

    let final_config: Config = app.wrap()
        .query_wasm_smart::<Config>(&sale_addr, &QueryMsg::GetConfig {})
        .unwrap();
    assert_eq!(final_config, initial_config, "Config should not change after FCFS round");

    assert!(total_amount_collected <= initial_config.total_allocation, "Total collected amount should not exceed total allocation");
}

#[test]
fn test_query_all_user_info_at_height_with_high_user_count() {
    let mut app = mock_app();
    let sale_addr = setup_test_case(&mut app);

    app.update_block(|block| {
        block.time = block.time.plus_seconds(40000);
    });

    let total_participants = 1000;
    let amount_per_participant = Uint128::new(10u128.mul(10u128.pow(6)));

    for i in 0..total_participants {
        let participant = format!("participant{}", i);
        app.sudo(cw_multi_test::SudoMsg::Bank(
            cw_multi_test::BankSudo::Mint {
                to_address: participant.clone(),
                amount: vec![coin(amount_per_participant.u128(), PAYMENT_DENOM)],
            },
        )).unwrap();

        let info = mock_info(&participant, &[coin(amount_per_participant.u128(), PAYMENT_DENOM)]);
        let join_result = join_fcfs_round(&mut app, &sale_addr, info, amount_per_participant);
        
        assert!(join_result.is_ok(), "Failed to join FCFS round for participant {}: {:?}", i, join_result.err());

        if i % 100 == 0 {
            app.update_block(next_block);
        }
    }

    app.update_block(next_block);

    let final_block_height = app.block_info().height;

    let mut all_users = Vec::new();
    let mut start_after: Option<String> = None;
    let limit = 100;

    loop {
        let msg = QueryMsg::GetAllUserInfoAtHeight { 
            start_after: start_after.clone(), 
            limit: Some(limit), 
            height: Some(final_block_height) 
        };
        let result: GetAllUserInfoAtHeightResponse = app.wrap().query_wasm_smart(&sale_addr, &msg).unwrap();
        
        all_users.extend(result.user_infos.clone());

        if result.user_infos.len() < limit as usize {
            break;
        }

        start_after = result.user_infos.last().map(|user_info| user_info.address.to_string());
    }

    assert_eq!(all_users.len(), total_participants as usize, "Should have retrieved all participants");

    let total_amount_collected: Uint128 = all_users.iter()
        .map(|user_info| user_info.total_payment_amount)
        .sum();

    assert_eq!(
        total_amount_collected, 
        amount_per_participant.mul(Uint128::new(total_participants)), 
        "Total collected amount should match"
    );

    for (index, user_info) in all_users.iter().enumerate() {
        assert!(user_info.joined_fcfs_round, "User {} should have joined FCFS round", index);
        assert_eq!(
            user_info.total_payment_amount, 
            amount_per_participant, 
            "User {} should have paid the correct amount", index
        );
    }

    let contract_balance = app.wrap().query_balance(sale_addr.as_str(), PAYMENT_DENOM).unwrap();
    assert_eq!(contract_balance.amount, total_amount_collected, "Contract balance should match total collected amount");

    let config: Config = app.wrap().query_wasm_smart(&sale_addr, &QueryMsg::GetConfig {}).unwrap();
    assert!(total_amount_collected <= config.total_allocation, "Total collected amount should not exceed total allocation");

    let statistics: Statistics = app.wrap().query_wasm_smart::<GetStatisticsResponse>(&sale_addr, &QueryMsg::GetStatistics { height: None }).unwrap().statistics;
    assert_eq!(statistics.total_fcfs_round_participants, Uint128::new(total_participants), "Total FCFS round participants should match");
    assert_eq!(statistics.total_payment_amount, total_amount_collected, "Total payment amount in statistics should match total collected amount");
}

#[test]
fn test_staker_round_with_100_users() {
    let mut app = mock_app();
    let sale_addr = setup_test_case(&mut app);

    let mut config: Config = app.wrap().query_wasm_smart(&sale_addr, &QueryMsg::GetConfig {}).unwrap();

    let total_participants = 100;
    let total_allocation = Uint128::new(100_000_000_000);

    config.dates.register_end = config.dates.register_end.checked_add(Uint64::new(100000)).unwrap();
    config.dates.staker_end = config.dates.staker_end.checked_add(Uint64::new(100000)).unwrap();
    config.total_allocation = total_allocation;

    let update_config_msg = ExecuteMsg::UpdateConfig {
        stake_controller: config.stake_controller.to_string(),
        payment_denom: config.payment_denom.clone(),
        sale_token_decimals: config.sale_token_decimals,
        sale_token_price: config.sale_token_price,
        min_allocation: config.min_allocation,
        total_allocation: total_allocation,
        fcfs_allocation: config.fcfs_allocation,
        status: config.status.clone(),
        dates: config.dates.clone(),
        whitelist_properties: config.whitelist_properties.clone(),
    };

    let owner_info = mock_info(OWNER, &[]);
    app.execute_contract(owner_info.sender, sale_addr.clone(), &update_config_msg, &[]).unwrap();

    let mut total_tokens_distributed = Uint128::zero();
    let mut total_staked = Uint128::zero();
    let base_stake = Uint128::new(1_000_000_000);

    for i in 0..total_participants {
        let participant = format!("participant{}", i);
        let stake_amount = base_stake * Uint128::new((i + 1) as u128);
        total_staked += stake_amount;

        app.sudo(cw_multi_test::SudoMsg::Bank(
            cw_multi_test::BankSudo::Mint {
                to_address: participant.clone(),
                amount: vec![coin(stake_amount.u128(), PAYMENT_DENOM)],
            },
        )).unwrap();

        let info = mock_info(&participant, &[]);
        
        app.update_block(|block| {
            block.time = Timestamp::from_seconds(config.dates.register_start.u64());
        });

        USER_STAKES.with(|stakes| {
            stakes.borrow_mut().insert(participant.clone(), stake_amount);
        });

        register_user(&mut app, &sale_addr, info.clone(), None).unwrap();
        app.update_block(next_block);
    }

    for i in 0..total_participants {
        let participant = format!("participant{}", i);
        let stake_amount = base_stake * Uint128::new((i + 1) as u128);
        
        let user_allocation = (total_allocation * stake_amount) / total_staked;

        let info = mock_info(&participant, &[coin(user_allocation.u128(), PAYMENT_DENOM)]);

        app.update_block(|block| {
            block.time = Timestamp::from_seconds(config.dates.staker_start.u64());
        });

        join_staker_round(&mut app, &sale_addr, info.clone(), user_allocation, None).unwrap();
        app.update_block(next_block);

        let user_info_after_staker = query_user_info(&app, sale_addr.clone(), participant.clone(), None);
        
        total_tokens_distributed += user_info_after_staker.total_payment_amount;

        let percentage_of_total = (user_info_after_staker.total_payment_amount.u128() as f64 / total_allocation.u128() as f64) * 100.0;

        eprintln!("Participant {}: Stake Amount: {}, Allocation: {}, Payment Amount: {}, Percentage of Total Allocation: {:.2}%", 
                 i, stake_amount, user_allocation, user_info_after_staker.total_payment_amount, percentage_of_total);

        if i == 0 || i == total_participants - 1 || i % 10 == 0 {
            let contract_balance = app.wrap().query_balance(sale_addr.as_str(), PAYMENT_DENOM).unwrap();
            let current_percentage = (contract_balance.amount.u128() as f64 / total_allocation.u128() as f64) * 100.0;
            eprintln!("Current Contract Balance: {}, Percentage of Total Allocation: {:.2}%", contract_balance.amount, current_percentage);
        }
    }

    let final_contract_balance = app.wrap().query_balance(sale_addr.as_str(), PAYMENT_DENOM).unwrap();
    let final_percentage = (final_contract_balance.amount.u128() as f64 / total_allocation.u128() as f64) * 100.0;

    eprintln!("Final Statistics:");
    eprintln!("Total Allocation: {}", total_allocation);
    eprintln!("Total Distributed: {}", total_tokens_distributed);
    eprintln!("Final Contract Balance: {}", final_contract_balance.amount);
    eprintln!("Percentage of Total Allocation Distributed: {:.2}%", final_percentage);

    assert!(total_tokens_distributed <= total_allocation, "Distributed tokens should not exceed total allocation");
    assert!(total_tokens_distributed >= total_allocation - Uint128::new(100), "At least 99.9999999% of tokens should be distributed");

    let statistics: GetStatisticsResponse = app.wrap().query_wasm_smart(&sale_addr, &QueryMsg::GetStatistics { height: None }).unwrap();
    assert_eq!(statistics.statistics.total_staker_round_participants, Uint128::new(total_participants), "Incorrect number of staker round participants");
    assert_eq!(statistics.statistics.total_payment_amount, total_tokens_distributed, "Incorrect total payment amount");

    eprintln!("All tokens were successfully distributed.");
}