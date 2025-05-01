use std::ops::Mul;

use crate::msg::{
    ExecuteMsg, QueryMsg, ReceiveMsg, GetRewardAmountResponse, GetStakedBalanceAtHeightResponse, GetTotalStakedAtHeightResponse
};
use crate::state::Config;
use crate::ContractError;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{to_json_binary, Addr, Empty, MessageInfo, Uint128, Uint64};
use cw20::Cw20Coin;
use cw_ownable::OwnershipError;
use cw_multi_test::{next_block, App, AppResponse, Contract, ContractWrapper, Executor};
use anyhow::Result as AnyResult;

const ADDR1: &str = "addr0001";
const OWNER: &str = "owner";

fn contract_stake() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

fn contract_cw20() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    );
    Box::new(contract)
}

fn mock_app() -> App {
    App::default()
}

fn get_balance<T: Into<String>, U: Into<String>>(
    app: &App,
    contract_addr: T,
    address: U,
) -> Uint128 {
    let msg = cw20::Cw20QueryMsg::Balance {
        address: address.into(),
    };
    let result: cw20::BalanceResponse = app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
    result.balance
}

fn instantiate_cw20(app: &mut App, initial_balances: Vec<Cw20Coin>) -> Addr {
    let cw20_id = app.store_code(contract_cw20());
    let msg = cw20_base::msg::InstantiateMsg {
        name: String::from("Test"),
        symbol: String::from("TEST"),
        decimals: 6,
        initial_balances,
        mint: None,
        marketing: None,
    };

    app.instantiate_contract(cw20_id, Addr::unchecked(ADDR1), &msg, &[], "cw20", None).unwrap()
}

fn instantiate_stake(app: &mut App, initial_token_address: Addr, initial_stake_paused: bool, initial_unstake_paused: bool, initial_emergency_unstake_paused: bool, initial_interest_rate: Uint64, initial_lock_duration: Uint64, initial_lock_duration_multiplier: Uint64, initial_emergency_unstake_fee_percentage: Uint64) -> Addr {
    let stake_code_id = app.store_code(contract_stake());
    let msg = crate::msg::InstantiateMsg {
        owner: Some(OWNER.to_string()),
        token_address: initial_token_address.to_string(),
        stake_paused: initial_stake_paused,
        unstake_paused: initial_unstake_paused,
        emergency_unstake_paused: initial_emergency_unstake_paused,
        interest_rate: initial_interest_rate,
        lock_duration: initial_lock_duration,
        lock_duration_multiplier: initial_lock_duration_multiplier,
        emergency_unstake_fee_percentage: initial_emergency_unstake_fee_percentage,
        fee_address: OWNER.to_string(),
    };
    app.instantiate_contract(
        stake_code_id,
        Addr::unchecked(ADDR1),
        &msg,
        &[],
        "stake",
        Some("admin".to_string()),
    )
    .unwrap()
}

fn setup_test_case(
    app: &mut App,
    initial_balances: Vec<Cw20Coin>,
    initial_stake_paused: bool, 
    initial_unstake_paused: bool, 
    initial_emergency_unstake_paused: bool,
    initial_interest_rate: Uint64, 
    initial_lock_duration: Uint64,
    initial_lock_duration_multiplier: Uint64,
    initial_emergency_unstake_fee_percentage: Uint64,
) -> (Addr, Addr) {
    let cw20_addr = instantiate_cw20(app, initial_balances);
    app.update_block(next_block);
    let stake_addr = instantiate_stake(app, cw20_addr.clone(), initial_stake_paused, initial_unstake_paused, initial_emergency_unstake_paused, initial_interest_rate, initial_lock_duration, initial_lock_duration_multiplier, initial_emergency_unstake_fee_percentage);
    app.update_block(next_block);
    (stake_addr, cw20_addr)
}

fn query_config<T: Into<String>>(app: &App, contract_addr: T) -> Config {
    let msg: QueryMsg = QueryMsg::GetConfig {};
    app.wrap().query_wasm_smart(contract_addr, &msg).unwrap()
}

fn query_reward_amount<T: Into<String>>(app: &App, contract_addr: T, address: String) -> Uint128 {
    let msg = QueryMsg::GetRewardAmount { address };
    let result: GetRewardAmountResponse = app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
    result.reward_amount
}

fn query_total_staked<T: Into<String>>(app: &App, contract_addr: T) -> Uint128 {
    let msg = QueryMsg::GetTotalStakedAtHeight { height: None };
    let result: GetTotalStakedAtHeightResponse =
        app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
    result.total
}

fn query_staked_balance<T: Into<String>, U: Into<String>>(
    app: &App,
    contract_addr: T,
    address: U,
) -> Uint128 {
    let msg = QueryMsg::GetStakedBalanceAtHeight {
        address: address.into(),
        height: None,
    };
    let result: GetStakedBalanceAtHeightResponse =
        app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
    result.balance
}

fn update_config(
    app: &mut App,
    stake_addr: &Addr,
    info: MessageInfo,
    stake_paused: bool, 
    unstake_paused: bool, 
    emergency_unstake_paused: bool,
    interest_rate: Uint64, 
    lock_duration: Uint64,
    lock_duration_multiplier: Uint64,
    emergency_unstake_fee_percentage: Uint64,
    fee_address: String,
) -> AnyResult<AppResponse> {
    let msg = ExecuteMsg::UpdateConfig { stake_paused, unstake_paused, emergency_unstake_paused, interest_rate, lock_duration, lock_duration_multiplier, emergency_unstake_fee_percentage, fee_address };
    app.execute_contract(info.sender, stake_addr.clone(), &msg, &[])
}

fn stake_tokens(
    app: &mut App,
    stake_addr: &Addr,
    cw20_addr: &Addr,
    info: MessageInfo,
    amount: Uint128,
) -> AnyResult<AppResponse> {
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: stake_addr.to_string(),
        amount,
        msg: to_json_binary(&ReceiveMsg::Stake {}).unwrap(),
    };
    app.execute_contract(info.sender, cw20_addr.clone(), &msg, &[])
}

fn add_tokens(
    app: &mut App,
    stake_addr: &Addr,
    cw20_addr: &Addr,
    info: MessageInfo,
    amount: Uint128,
) -> AnyResult<AppResponse> {
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: stake_addr.to_string(),
        amount,
        msg: to_json_binary(&ReceiveMsg::AddTokens {}).unwrap(),
    };
    app.execute_contract(info.sender, cw20_addr.clone(), &msg, &[])
}

fn unstake_tokens(
    app: &mut App,
    stake_addr: &Addr,
    info: MessageInfo,
    amount: Uint128,
) -> AnyResult<AppResponse> {
    let msg = ExecuteMsg::Unstake { amount };
    app.execute_contract(info.sender, stake_addr.clone(), &msg, &[])
}

fn emergency_unstake_tokens(
    app: &mut App,
    stake_addr: &Addr,
    info: MessageInfo,
    amount: Uint128,
) -> AnyResult<AppResponse> {
    let msg = ExecuteMsg::EmergencyUnstake { amount };
    app.execute_contract(info.sender, stake_addr.clone(), &msg, &[])
}

fn reinvest_rewards(
    app: &mut App,
    stake_addr: &Addr,
    info: MessageInfo,
) -> AnyResult<AppResponse> {
    let msg = ExecuteMsg::ReInvest {};
    app.execute_contract(info.sender, stake_addr.clone(), &msg, &[])
}

fn harvest_rewards(
    app: &mut App,
    stake_addr: &Addr,
    info: MessageInfo,
) -> AnyResult<AppResponse> {
    let msg = ExecuteMsg::Harvest {};
    app.execute_contract(info.sender, stake_addr.clone(), &msg, &[])
}

#[test]
fn test_update_config() {
    let mut app = mock_app();
    let amount1 = Uint128::from(1100u128.mul(10u128.pow(6)));
    let initial_balances = vec![Cw20Coin {
        address: ADDR1.to_string(),
        amount: amount1,
    }];
    let (stake_addr, _cw20_addr) = setup_test_case(&mut app, initial_balances, false, false, false, Uint64::from(500u64), Uint64::from(14u64 * 24u64 * 60u64 * 60u64), Uint64::from(10000u64), Uint64::from(500u64));

    let info = mock_info(OWNER, &[]);
    update_config(&mut app, &stake_addr, info, false, true,  false, Uint64::from(500u64), Uint64::from(14u64 * 24u64 * 60u64 * 60u64), Uint64::from(10000u64), Uint64::from(500u64), ADDR1.to_string()).unwrap();
    let config = query_config(&app, &stake_addr);
    assert_eq!(config.interest_rate, Uint64::from(500u64));

    let info = mock_info(ADDR1, &[]);
    let err: ContractError = update_config(&mut app, &stake_addr, info, true, false,  false, Uint64::from(500u64), Uint64::from(14u64 * 24u64 * 60u64 * 60u64), Uint64::from(10000u64), Uint64::from(500u64), ADDR1.to_string())
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::Ownership(OwnershipError::NotOwner));

    let info = mock_info(OWNER, &[]);
    let err: ContractError =
        update_config(&mut app, &stake_addr, info, false, false, false, Uint64::from(0u64), Uint64::from(14u64 * 24u64 * 60u64 * 60u64), Uint64::from(10000u64), Uint64::from(500u64), ADDR1.to_string())
            .unwrap_err()
            .downcast()
            .unwrap();
    assert_eq!(err, ContractError::InvalidInterestRate {});
}

#[test]
fn test_stake() {
    let _deps = mock_dependencies();

    let mut app = mock_app();
    let amount1 = Uint128::from(1100u128.mul(10u128.pow(6)));
    let _token_address = Addr::unchecked("token_address");
    let initial_balances = vec![Cw20Coin {
        address: ADDR1.to_string(),
        amount: amount1,
    }];
    let (stake_addr, cw20_addr) = setup_test_case(&mut app, initial_balances, false, false, false, Uint64::from(500u64), Uint64::from(14u64 * 24u64 * 60u64 * 60u64), Uint64::from(10000u64), Uint64::from(500u64));

    let info = mock_info(ADDR1, &[]);
    let _env = mock_env();

    let reward_token_amount = Uint128::from(1000u128.mul(10u128.pow(6)));
    add_tokens(&mut app, &stake_addr, &cw20_addr, info.clone(), reward_token_amount).unwrap();
    app.update_block(next_block);

    let amount = Uint128::new(50u128.mul(10u128.pow(6)));
    stake_tokens(&mut app, &stake_addr, &cw20_addr, info.clone(), amount).unwrap();

    assert_eq!(
        query_staked_balance(&app, &stake_addr, ADDR1.to_string()),
        Uint128::zero()
    );

    app.update_block(next_block);

    assert_eq!(
        query_staked_balance(&app, &stake_addr, ADDR1.to_string()),
        Uint128::from(50u128.mul(10u128.pow(6)))
    );
    assert_eq!(
        query_total_staked(&app, &stake_addr),
        Uint128::from(50u128.mul(10u128.pow(6)))
    );
    assert_eq!(
        get_balance(&app, &cw20_addr, ADDR1.to_string()),
        Uint128::from(50u128.mul(10u128.pow(6)))
    );

    stake_tokens(&mut app, &stake_addr, &cw20_addr, info.clone(), amount).unwrap();
    app.update_block(next_block);

    assert_eq!(
        query_staked_balance(&app, &stake_addr, ADDR1.to_string()),
        Uint128::from(100u128.mul(10u128.pow(6)))
    );
    assert_eq!(
        query_total_staked(&app, &stake_addr),
        Uint128::from(100u128.mul(10u128.pow(6)))
    );
    assert_eq!(
        get_balance(&app, &cw20_addr, ADDR1.to_string()),
        Uint128::from(0u128.mul(10u128.pow(6)))
    );
}

#[test]
fn test_unstake() {
    let _deps = mock_dependencies();

    let mut app = mock_app();
    let amount1 = Uint128::from(1100u128.mul(10u128.pow(6)));
    let _token_address = Addr::unchecked("token_address");
    let initial_balances = vec![Cw20Coin {
        address: ADDR1.to_string(),
        amount: amount1,
    }];
    let (stake_addr, cw20_addr) = setup_test_case(&mut app, initial_balances, false, false, false, Uint64::from(500u64), Uint64::from(14u64 * 24u64 * 60u64 * 60u64), Uint64::from(10000u64), Uint64::from(500u64));

    let info: MessageInfo = mock_info(ADDR1, &[]);
    let _env: cosmwasm_std::Env = mock_env();

    let reward_token_amount = Uint128::from(1000u128.mul(10u128.pow(6)));
    add_tokens(&mut app, &stake_addr, &cw20_addr, info.clone(), reward_token_amount).unwrap();
    app.update_block(next_block);

    let amount = Uint128::new(50u128.mul(10u128.pow(6)));
    stake_tokens(&mut app, &stake_addr, &cw20_addr, info.clone(), amount).unwrap();

    assert_eq!(
        query_staked_balance(&app, &stake_addr, ADDR1.to_string()),
        Uint128::zero()
    );

    app.update_block(next_block);

    assert_eq!(
        query_staked_balance(&app, &stake_addr, ADDR1.to_string()),
        Uint128::from(50u128.mul(10u128.pow(6)))
    );
    assert_eq!(
        query_total_staked(&app, &stake_addr),
        Uint128::from(50u128.mul(10u128.pow(6)))
    );
    assert_eq!(
        get_balance(&app, &cw20_addr, ADDR1.to_string()),
        Uint128::from(50u128.mul(10u128.pow(6)))
    );

    stake_tokens(&mut app, &stake_addr, &cw20_addr, info.clone(), amount).unwrap();
    app.update_block(next_block);

    assert_eq!(
        query_staked_balance(&app, &stake_addr, ADDR1.to_string()),
        Uint128::from(100u128.mul(10u128.pow(6)))
    );
    assert_eq!(
        query_total_staked(&app, &stake_addr),
        Uint128::from(100u128.mul(10u128.pow(6)))
    );
    assert_eq!(
        get_balance(&app, &cw20_addr, ADDR1.to_string()),
        Uint128::from(0u128.mul(10u128.pow(6)))
    );

    let lock_duration = 15u64 * 24u64 * 60u64 * 60u64;

    app.update_block(|b| {
        b.time = b.time.plus_seconds(lock_duration);
    });

    unstake_tokens(&mut app, &stake_addr, info.clone(), Uint128::from(10u128.mul(10u128.pow(6)))).unwrap();
    app.update_block(next_block);

    assert_eq!(
        query_staked_balance(&app, &stake_addr, ADDR1.to_string()),
        Uint128::from(90u128.mul(10u128.pow(6)))
    )
}
#[test]
fn test_emergency_unstake() {
    let _deps = mock_dependencies();

    let mut app = mock_app();
    let amount1 = Uint128::from(1100u128.mul(10u128.pow(6)));
    let _token_address = Addr::unchecked("token_address");
    let initial_balances = vec![Cw20Coin {
        address: ADDR1.to_string(),
        amount: amount1,
    }];
    let (stake_addr, cw20_addr) = setup_test_case(&mut app, initial_balances, false, false, false, Uint64::from(500u64), Uint64::from(14u64 * 24u64 * 60u64 * 60u64), Uint64::from(10000u64), Uint64::from(500u64));

    let info = mock_info(ADDR1, &[]);
    let _env = mock_env();

    let reward_token_amount = Uint128::from(1000u128.mul(10u128.pow(6)));
    add_tokens(&mut app, &stake_addr, &cw20_addr, info.clone(), reward_token_amount).unwrap();
    app.update_block(next_block);

    let amount = Uint128::new(50u128.mul(10u128.pow(6)));
    stake_tokens(&mut app, &stake_addr, &cw20_addr, info.clone(), amount).unwrap();
    app.update_block(next_block);

    let unstake_amount = Uint128::new(30u128.mul(10u128.pow(6)));
    emergency_unstake_tokens(&mut app, &stake_addr, info.clone(), unstake_amount).unwrap();
    app.update_block(next_block);

    assert_eq!(
        query_staked_balance(&app, &stake_addr, ADDR1.to_string()),
        Uint128::from(20u128.mul(10u128.pow(6)))
    );

    assert_eq!(
        query_total_staked(&app, &stake_addr),
        Uint128::from(20u128.mul(10u128.pow(6))) 
    );

    let fee_percentage = 500u64; 
    let fee_amount = unstake_amount.multiply_ratio(fee_percentage, 10000u128);
    let expected_balance = Uint128::from(100u128.mul(10u128.pow(6))) - Uint128::from(50u128.mul(10u128.pow(6))) + (unstake_amount - fee_amount);

    assert_eq!(
        get_balance(&app, &cw20_addr, ADDR1.to_string()),
        expected_balance
    );
}

#[test]
fn test_harvest() {
    let _deps = mock_dependencies();

    let mut app = mock_app();
    let amount1 = Uint128::from(1100u128 * 10u128.pow(6));
    let _token_address = Addr::unchecked("token_address");
    let initial_balances = vec![Cw20Coin {
        address: ADDR1.to_string(),
        amount: amount1,
    }];
    let (stake_addr, cw20_addr) = setup_test_case(&mut app, initial_balances, false, false, false, Uint64::from(500u64), Uint64::from(14u64 * 24u64 * 60u64 * 60u64), Uint64::from(10000u64), Uint64::from(500u64));

    let info = mock_info(ADDR1, &[]);
    let _env = mock_env();

    let reward_token_amount = Uint128::from(1000u128.mul(10u128.pow(6)));
    add_tokens(&mut app, &stake_addr, &cw20_addr, info.clone(), reward_token_amount).unwrap();
    app.update_block(next_block);

    let amount = Uint128::new(50u128 * 10u128.pow(6));
    stake_tokens(&mut app, &stake_addr, &cw20_addr, info.clone(), amount).unwrap();
    app.update_block(next_block);

    let lock_duration = 15u64 * 24u64 * 60u64 * 60u64;

    app.update_block(|b| {
        b.time = b.time.plus_seconds(lock_duration);
    });

    let reward_amount = query_reward_amount(&app, &stake_addr, ADDR1.to_string());

    harvest_rewards(&mut app, &stake_addr, info.clone()).unwrap();
    app.update_block(next_block);

    assert_eq!(
        query_staked_balance(&app, &stake_addr, ADDR1.to_string()),
        Uint128::from(50u128 * 10u128.pow(6))
    );

    let expected_balance = Uint128::from(50u128 * 10u128.pow(6)) + reward_amount;

    assert_eq!(
        get_balance(&app, &cw20_addr, ADDR1.to_string()),
        expected_balance
    );
}

#[test]
fn test_reinvest() {
    let _deps = mock_dependencies();

    let mut app = mock_app();
    let amount1 = Uint128::from(1100u128 * 10u128.pow(6));
    let _token_address = Addr::unchecked("token_address");
    let initial_balances = vec![Cw20Coin {
        address: ADDR1.to_string(),
        amount: amount1,
    }];
    let (stake_addr, cw20_addr) = setup_test_case(&mut app, initial_balances, false, false, false, Uint64::from(500u64), Uint64::from(14u64 * 24u64 * 60u64 * 60u64), Uint64::from(10000u64), Uint64::from(500u64));

    let info = mock_info(ADDR1, &[]);
    let _env = mock_env();

    let reward_token_amount = Uint128::from(1000u128.mul(10u128.pow(6)));
    add_tokens(&mut app, &stake_addr, &cw20_addr, info.clone(), reward_token_amount).unwrap();
    app.update_block(next_block);

    let amount = Uint128::new(50u128 * 10u128.pow(6));
    stake_tokens(&mut app, &stake_addr, &cw20_addr, info.clone(), amount).unwrap();
    app.update_block(next_block);

    let lock_duration = 15u64 * 24u64 * 60u64 * 60u64;

    app.update_block(|b| {
        b.time = b.time.plus_seconds(lock_duration);
    });

    let reward_amount = query_reward_amount(&app, &stake_addr, ADDR1.to_string());

    reinvest_rewards(&mut app, &stake_addr, info.clone()).unwrap();
    app.update_block(next_block);

    assert_eq!(
        query_staked_balance(&app, &stake_addr, ADDR1.to_string()),
        Uint128::from(50u128 * 10u128.pow(6)) + reward_amount
    );

    let expected_balance = Uint128::from(50u128 * 10u128.pow(6));

    assert_eq!(
        get_balance(&app, &cw20_addr, ADDR1.to_string()),
        expected_balance
    );
}