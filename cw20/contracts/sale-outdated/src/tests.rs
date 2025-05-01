use std::ops::Mul;
use crate::msg::{
    ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg, GetUserInfoAtHeightResponse, GetUserTierResponse
};
use crate::state::{Dates, Status, UserInfo, WhitelistProperties};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{to_json_binary, Addr, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult, Uint128, Uint64};
use cw20::Cw20Coin;
use cw20_base::ContractError;
use cw_multi_test::{next_block, App, AppResponse, Contract, ContractWrapper, Executor};
use anyhow::Result as AnyResult;

const ADDR1: &str = "addr0001";
const OWNER: &str = "owner";
const PAYMENT_TOKEN_DECIMALS: u8 = 6;
const SALE_TOKEN_DECIMALS: u8 = 18;
const STAKE_TOKEN_DECIMALS: u8 = 6;

fn contract_sale() -> Box<dyn Contract<Empty>> {
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
        MockQueryMsg::GetUserTierIndex { address: _ } => {
            let response = GetUserTierResponse { tier: Uint64::new(1), total_staked: Uint128::new(100u128.mul(10u128.pow(u32::from(STAKE_TOKEN_DECIMALS)))) };
            to_json_binary(&response)
        },
        MockQueryMsg::GetUserStake { address: _ } => {
            let response = GetUserStakeResponse { stake: Uint128::new(100u128.mul(10u128.pow(u32::from(STAKE_TOKEN_DECIMALS)))) };
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

fn instantiate_cw20(app: &mut App, initial_balances: Vec<Cw20Coin>, decimals: u8) -> Addr {
    let cw20_id = app.store_code(contract_cw20());
    let msg = cw20_base::msg::InstantiateMsg {
        name: String::from("Test"),
        symbol: String::from("TEST"),
        decimals,
        initial_balances,
        mint: None,
        marketing: None,
    };

    app.instantiate_contract(cw20_id, Addr::unchecked(ADDR1), &msg, &[], "cw20", None).unwrap()
}

fn instantiate_sale(app: &mut App, payment_token_address: Addr, stake_controller_address: Addr) -> Addr {
    let sale_code_id = app.store_code(contract_sale());
    let msg = InstantiateMsg {
        owner: Some(OWNER.to_string()),
        stake_controller: stake_controller_address.to_string(),
        payment_token: payment_token_address.to_string(),
        sale_token_decimals: Uint64::new(6),
        sale_token_price: Uint128::new(1u128.mul(10u128.pow(5))),
        min_allocation: Uint128::new(10u128.mul(10u128.pow(6))),
        total_allocation: Uint128::new(1000000u128.mul(10u128.pow(6))),
        fcfs_multiplier: Uint64::new(1500),
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

fn setup_test_case(app: &mut App, initial_balances: Vec<Cw20Coin>) -> (Addr, Addr, Addr) {
    let payment_token_addr = instantiate_cw20(app, initial_balances.clone(), PAYMENT_TOKEN_DECIMALS);
    app.update_block(next_block);

    let sale_token_addr = instantiate_cw20(app, initial_balances.clone(), SALE_TOKEN_DECIMALS);
    app.update_block(next_block);
    
    let stake_controller_addr = instantiate_stake_controller(app);
    app.update_block(next_block);
    
    let sale_addr = instantiate_sale(app, payment_token_addr.clone(), stake_controller_addr.clone());
    app.update_block(next_block);
    
    (sale_addr, payment_token_addr, sale_token_addr)
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
    cw20_addr: &Addr,
    info: MessageInfo,
    amount: Uint128,
    proof: Option<Vec<String>>,
) -> AnyResult<AppResponse> {
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: sale_addr.to_string(),
        amount,
        msg: to_json_binary(&(ReceiveMsg::JoinStakerRound { proof })).unwrap(),
    };
    app.execute_contract(info.sender, cw20_addr.clone(), &msg, &[])
}

fn join_fcfs_round(
    app: &mut App,
    sale_addr: &Addr,
    cw20_addr: &Addr,
    info: MessageInfo,
    amount: Uint128,
) -> AnyResult<AppResponse> {
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: sale_addr.to_string(),
        amount,
        msg: to_json_binary(&(ReceiveMsg::JoinFcfsRound {})).unwrap(),
    };
    app.execute_contract(info.sender, cw20_addr.clone(), &msg, &[])
}

fn query_user_info<T: Into<String>>(app: &App, contract_addr: T, address: String, height: Option<u64>) -> UserInfo {
    let msg = QueryMsg::GetUserInfoAtHeight { address, height };
    let result: GetUserInfoAtHeightResponse = app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
    result.user_info
}

#[test]
fn test_register_user() {
    let mut app = mock_app();
    let amount1 = Uint128::from(1100u128.mul(10u128.pow(6)));
    let initial_balances = vec![Cw20Coin {
        address: ADDR1.to_string(),
        amount: amount1,
    }];
    let (sale_addr, _payment_token_addr, _sale_token_addr) = setup_test_case(&mut app, initial_balances);

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
    let amount1 = Uint128::from(1100u128.mul(10u128.pow(6)));
    let initial_balances = vec![Cw20Coin {
        address: ADDR1.to_string(),
        amount: amount1,
    }];
    let (sale_addr, payment_token_addr, _sale_token_addr) = setup_test_case(&mut app, initial_balances);
    let info = mock_info(ADDR1, &[]);

    register_user(&mut app, &sale_addr, info.clone(), None).unwrap();
    app.update_block(next_block);

    app.update_block(|block| {
        block.time = block.time.plus_seconds(20000);
    });
    
    let amount = Uint128::new(100u128.mul(10u128.pow(6)));
    join_staker_round(&mut app, &sale_addr, &payment_token_addr, info.clone(), amount, None).unwrap();
    app.update_block(next_block);

    let block_height = app.block_info().height;
    let user_info = query_user_info(&app, sale_addr, ADDR1.to_string(), Some(block_height));
    assert!(user_info.joined_staker_round);
}

#[test]
fn test_join_fcfs_round() {
    let mut app = mock_app();
    let amount1 = Uint128::from(1100u128.mul(10u128.pow(6)));
    let initial_balances = vec![Cw20Coin {
        address: ADDR1.to_string(),
        amount: amount1,
    }];    
    let (sale_addr, payment_token_addr, _sale_token_addr) = setup_test_case(&mut app, initial_balances);
    let info = mock_info(ADDR1, &[]);

    app.update_block(|block| {
        block.time = block.time.plus_seconds(40000);
    });
    
    let amount = Uint128::new(100u128.mul(10u128.pow(6)));
    join_fcfs_round(&mut app, &sale_addr, &payment_token_addr, info.clone(), amount).unwrap();
    app.update_block(next_block);

    let block_height = app.block_info().height;
    let user_info = query_user_info(&app, sale_addr, ADDR1.to_string(), Some(block_height));
    assert!(user_info.joined_fcfs_round);
}