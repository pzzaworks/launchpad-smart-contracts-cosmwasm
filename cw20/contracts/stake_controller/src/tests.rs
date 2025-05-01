use std::ops::Mul;
use cosmwasm_std::{to_json_binary, Addr, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult, Uint128, Uint64};
use cw20_base::ContractError;
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use crate::msg::{InstantiateMsg, QueryMsg, GetUserTierResponse, GetStakedValueResponse};

fn contract_cw20() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    );
    Box::new(contract)
}

fn contract_stake() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mock_stake_execute,
        mock_stake_instantiate,
        mock_stake_query,
    );
    Box::new(contract)
}

fn contract_stake_controller() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

fn mock_app() -> App {
    App::default()
}

fn mock_stake_query(
    _deps: Deps,
    _env: Env,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetStakedValue { address: _ } => {
            let response = GetStakedValueResponse { value: Uint128::new(100u128.mul(10u128.pow(6))) };
            to_json_binary(&response)
        },
        _ => panic!("Unsupported query"),
    }
}

fn mock_stake_instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
) -> Result<Response, ContractError> {
    Ok(Response::new().add_attribute("method", "instantiate"))
}

fn mock_stake_execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
) -> Result<Response, ContractError> {
    Ok(Response::new().add_attribute("method", "execute"))
}

fn setup_test_case() -> (App, Addr) {
    let mut app = mock_app();

    let cw20_code_id = app.store_code(contract_cw20());
    let stake_code_id = app.store_code(contract_stake());
    let stake_controller_code_id = app.store_code(contract_stake_controller());

    let cw20_msg = cw20_base::msg::InstantiateMsg {
        name: "Test Token".to_string(),
        symbol: "TST".to_string(),
        decimals: 6,
        initial_balances: vec![cw20::Cw20Coin {
            address: "owner0000".to_string(),
            amount: Uint128::new(100 * 10u128.pow(6)),
        }],
        mint: None,
        marketing: None,
    };

    let cw20_addr = app
        .instantiate_contract(
            cw20_code_id,
            Addr::unchecked("owner0000"),
            &cw20_msg,
            &[],
            "cw20",
            None,
        )
        .unwrap();

    let stake1_addr = app
        .instantiate_contract(
            stake_code_id,
            Addr::unchecked("owner0000"),
            &Empty {},
            &[],
            "stake1",
            None,
        )
        .unwrap();

    let stake2_addr = app
        .instantiate_contract(
            stake_code_id,
            Addr::unchecked("owner0000"),
            &Empty {},
            &[],
            "stake2",
            None,
        )
        .unwrap();

    let msg = InstantiateMsg {
        owner: Some("owner0000".to_string()),
        token_address: cw20_addr.to_string(),
        stake_contracts: vec![stake1_addr.to_string(), stake2_addr.to_string()],
        stake_contract_multipliers: vec![Uint64::new(10000), Uint64::new(20000)],
        tier_thresholds: vec![Uint128::new(100u128.mul(10u128.pow(6))), Uint128::new(200u128.mul(10u128.pow(6)))],
    };

    let stake_controller_addr = app
        .instantiate_contract(
            stake_controller_code_id,
            Addr::unchecked("owner0000"),
            &msg,
            &[],
            "stake_controller",
            None,
        )
        .unwrap();

    (app, stake_controller_addr)
}

#[test]
fn test_query_user_tier_index() {
    let (app, stake_controller_addr) = setup_test_case();

    let address = "user0001".to_string();

    let res: GetUserTierResponse = app
        .wrap()
        .query_wasm_smart(
            stake_controller_addr.clone(),
            &QueryMsg::GetUserTierIndex {
                address: address.clone(),
            },
        )
        .unwrap();

    assert_eq!(res.tier, Uint64::new(2));
    assert_eq!(res.total_staked, Uint128::new(300u128.mul(10u128.pow(6)))); 
}

#[test]
fn test_query_total_staked() {
    let (app, stake_controller_addr) = setup_test_case();

    let address = "user0001".to_string();

    let res: Uint128 = app
        .wrap()
        .query_wasm_smart(
            stake_controller_addr.clone(),
            &QueryMsg::GetTotalStaked {
                address: address.clone(),
            },
        )
        .unwrap();

    assert_eq!(res, Uint128::new(300u128.mul(10u128.pow(6))));
}

#[test]
fn test_update_custom_tiers() {
    let (mut app, stake_controller_addr) = setup_test_case();

    let address = "user0001".to_string();

    let msg = crate::msg::ExecuteMsg::UpdateCustomTiers {
        address: address.clone(),
        tier_index: Uint64::new(2),
    };

    app.execute_contract(Addr::unchecked("owner0000"), stake_controller_addr.clone(), &msg, &[])
        .unwrap();

    let res: GetUserTierResponse = app
        .wrap()
        .query_wasm_smart(
            stake_controller_addr.clone(),
            &QueryMsg::GetUserTierIndex {
                address: address.clone(),
            },
        )
        .unwrap();

    assert_eq!(res.tier, Uint64::new(2));
    assert_eq!(res.total_staked, Uint128::zero());
}