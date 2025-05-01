use crate::msg::{ExecuteMsg, GetConfigResponse, GetLastClaimResponse, GetTotalClaimsResponse, InstantiateMsg, QueryMsg, ReceiveMsg};
use crate::state::{TokenConfig, NativeCoinConfig};
use cosmwasm_std::{coin, to_json_binary, Addr, Empty, Uint128, Uint64};
use cw20::Cw20Coin;
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
const OWNER: &str = "owner";
const USER1: &str = "user1";
const TOKEN1_DECIMALS: u8 = 6;
const TOKEN2_DECIMALS: u8 = 18;

fn mock_app() -> App {
    App::default()
}

fn contract_faucet() -> Box<dyn Contract<Empty>> {
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

fn instantiate_cw20(app: &mut App, initial_balances: Vec<Cw20Coin>, decimals: u8) -> Addr {
    let cw20_id = app.store_code(contract_cw20());
    let msg = cw20_base::msg::InstantiateMsg {
        name: String::from("Test Token"),
        symbol: String::from("TEST"),
        decimals,
        initial_balances,
        mint: Some(cw20::MinterResponse {
            minter: OWNER.to_string(),
            cap: None,
        }),
        marketing: None,
    };

    app.instantiate_contract(cw20_id, Addr::unchecked(OWNER), &msg, &[], "cw20", None).unwrap()
}

fn instantiate_faucet(
    app: &mut App,
    token1_address: Addr,
    token2_address: Addr,
) -> Addr {
    let faucet_code_id = app.store_code(contract_faucet());
    let msg = InstantiateMsg {
        owner: Some(OWNER.to_string()),
        tokens: vec![
            TokenConfig {
                address: token1_address.to_string(),
                amount: Uint128::new(100u128 * 10u128.pow(u32::from(TOKEN1_DECIMALS))),
            },
            TokenConfig {
                address: token2_address.to_string(),
                amount: Uint128::new(100u128 * 10u128.pow(u32::from(TOKEN2_DECIMALS))),
            },
        ],
        native_coin: NativeCoinConfig {
            denom: "uusd".to_string(),
            amount: Uint128::new(100_000), // 0.1 UUSD (assuming 6 decimals)
        },
        claim_interval: Uint64::new(24 * 60 * 60), 
    };
    app.instantiate_contract(
        faucet_code_id,
        Addr::unchecked(OWNER),
        &msg,
        &[],
        "faucet",
        None,
    ).unwrap()
}

fn setup_test_case(app: &mut App) -> (Addr, Addr, Addr) {
    let initial_balance_token1 = vec![
        Cw20Coin {
            address: OWNER.to_string(),
            amount: Uint128::new(1_000_000 * 10u128.pow(u32::from(TOKEN1_DECIMALS))),
        },
    ];
    let initial_balance_token2 = vec![
        Cw20Coin {
            address: OWNER.to_string(),
            amount: Uint128::new(1_000_000 * 10u128.pow(u32::from(TOKEN2_DECIMALS))),
        },
    ];
    let token1_addr = instantiate_cw20(app, initial_balance_token1.clone(), TOKEN1_DECIMALS);
    let token2_addr = instantiate_cw20(app, initial_balance_token2.clone(), TOKEN2_DECIMALS);
    
    let faucet_addr = instantiate_faucet(app, token1_addr.clone(), token2_addr.clone());
    
    let add_tokens_msg1: cw20::Cw20ExecuteMsg = cw20::Cw20ExecuteMsg::Send {
        contract: faucet_addr.to_string(),
        amount: Uint128::new(10_000 * 10u128.pow(u32::from(TOKEN1_DECIMALS))), // 10,000 Token1
        msg: to_json_binary(&ReceiveMsg::AddTokens {}).unwrap(),
    };
    let add_tokens_msg2 = cw20::Cw20ExecuteMsg::Send {
        contract: faucet_addr.to_string(),
        amount: Uint128::new(10_000 * 10u128.pow(u32::from(TOKEN2_DECIMALS))), // 10,000 Token2
        msg: to_json_binary(&ReceiveMsg::AddTokens {}).unwrap(),
    };
    app.execute_contract(Addr::unchecked(OWNER), token1_addr.clone(), &add_tokens_msg1, &[]).unwrap();
    app.execute_contract(Addr::unchecked(OWNER), token2_addr.clone(), &add_tokens_msg2, &[]).unwrap();

    app.init_modules(|router, _, storage| {
        router.bank.init_balance(
            storage,
            &Addr::unchecked(OWNER),
            vec![coin(1_000 * 10u128.pow(6u32), "uusd")], // 1000 UUSD
        ).unwrap();
    });

    let add_native_msg = ExecuteMsg::AddNativeTokens {};
    let native_coin = coin(1000 * 10u128.pow(6u32), "uusd"); // 1000 UUSD
    app.execute_contract(Addr::unchecked(OWNER), faucet_addr.clone(), &add_native_msg, &[native_coin]).unwrap();
    
    (faucet_addr, token1_addr, token2_addr)
}

#[test]
fn test_update_config() {
    let mut app = mock_app();
    let (faucet_addr, token1_addr, token2_addr) = setup_test_case(&mut app);

    let update_msg = ExecuteMsg::UpdateConfig {
        tokens: Some(vec![
            TokenConfig {
                address: token1_addr.to_string(),
                amount: Uint128::new(500 * 10u128.pow(u32::from(TOKEN1_DECIMALS))),
            },
            TokenConfig {
                address: token2_addr.to_string(),
                amount: Uint128::new(750 * 10u128.pow(u32::from(TOKEN2_DECIMALS))),
            },
        ]),
        native_coin: Some(NativeCoinConfig {
            denom: "uusd".to_string(),
            amount: Uint128::new(50 * 10u128.pow(6u32)), // 50 UUSD
        }),
        claim_interval: Some(Uint64::new(43200)), // 12 hours
    };
    let res = app.execute_contract(Addr::unchecked(OWNER), faucet_addr.clone(), &update_msg, &[]);
    assert!(res.is_ok());

    let config: GetConfigResponse = app.wrap().query_wasm_smart(&faucet_addr, &QueryMsg::GetConfig {}).unwrap();
    assert_eq!(config.tokens[0].amount, Uint128::new(500 * 10u128.pow(u32::from(TOKEN1_DECIMALS))));
    assert_eq!(config.tokens[1].amount, Uint128::new(750 * 10u128.pow(u32::from(TOKEN2_DECIMALS))));
    assert_eq!(config.native_coin.amount, Uint128::new(50 * 10u128.pow(6u32)));
    assert_eq!(config.claim_interval, Uint64::new(43200));

    let unauthorized_msg = ExecuteMsg::UpdateConfig {
        tokens: None,
        native_coin: Some(NativeCoinConfig {
            denom: "uusd".to_string(),
            amount: Uint128::new(60 * 10u128.pow(6u32)),
        }),
        claim_interval: None,
    };
    let err = app.execute_contract(Addr::unchecked(USER1), faucet_addr.clone(), &unauthorized_msg, &[])
        .unwrap_err();
    assert!(err.root_cause().to_string().contains("Caller is not the contract's current owner"));

    let invalid_interval_msg = ExecuteMsg::UpdateConfig {
        tokens: None,
        native_coin: None,
        claim_interval: Some(Uint64::zero()),
    };
    let err = app.execute_contract(Addr::unchecked(OWNER), faucet_addr.clone(), &invalid_interval_msg, &[])
        .unwrap_err();
    assert!(err.root_cause().to_string().contains("Invalid claim interval"));

    let zero_token_amount_msg = ExecuteMsg::UpdateConfig {
        tokens: Some(vec![
            TokenConfig {
                address: token1_addr.to_string(),
                amount: Uint128::zero(),
            },
        ]),
        native_coin: None,
        claim_interval: None,
    };
    let err = app.execute_contract(Addr::unchecked(OWNER), faucet_addr.clone(), &zero_token_amount_msg, &[])
        .unwrap_err();
    assert!(err.root_cause().to_string().contains("Invalid token amount"));

    let empty_denom_msg = ExecuteMsg::UpdateConfig {
        tokens: None,
        native_coin: Some(NativeCoinConfig {
            denom: "".to_string(),
            amount: Uint128::new(100),
        }),
        claim_interval: None,
    };
    let err = app.execute_contract(Addr::unchecked(OWNER), faucet_addr.clone(), &empty_denom_msg, &[])
        .unwrap_err();
    assert!(err.root_cause().to_string().contains("Invalid native coin denom"));

    let zero_native_amount_msg = ExecuteMsg::UpdateConfig {
        tokens: None,
        native_coin: Some(NativeCoinConfig {
            denom: "uusd".to_string(),
            amount: Uint128::zero(),
        }),
        claim_interval: None,
    };
    let err = app.execute_contract(Addr::unchecked(OWNER), faucet_addr.clone(), &zero_native_amount_msg, &[])
        .unwrap_err();
    assert!(err.root_cause().to_string().contains("Invalid native coin amount"));
}

#[test]
fn test_execute_claim() {
    let mut app = mock_app();
    let (faucet_addr, token1_addr, token2_addr) = setup_test_case(&mut app);

    let claim_msg = ExecuteMsg::Claim {};
    let res = app.execute_contract(Addr::unchecked(USER1), faucet_addr.clone(), &claim_msg, &[]);
    assert!(res.is_ok());

    let balance1: cw20::BalanceResponse = app.wrap().query_wasm_smart(
        &token1_addr,
        &cw20::Cw20QueryMsg::Balance { address: USER1.to_string() },
    ).unwrap();
    assert_eq!(balance1.balance, Uint128::new(100 * 10u128.pow(u32::from(TOKEN1_DECIMALS))));

    let balance2: cw20::BalanceResponse = app.wrap().query_wasm_smart(
        &token2_addr,
        &cw20::Cw20QueryMsg::Balance { address: USER1.to_string() },
    ).unwrap();
    assert_eq!(balance2.balance, Uint128::new(100 * 10u128.pow(u32::from(TOKEN2_DECIMALS))));

    let native_balance = app.wrap().query_balance(USER1, "uusd").unwrap();
    assert_eq!(native_balance.amount, Uint128::new(100_000)); // 0.1 UUSD

    let err = app.execute_contract(Addr::unchecked(USER1), faucet_addr.clone(), &claim_msg, &[]).unwrap_err();
    assert!(err.root_cause().to_string().contains("Claim too early"));

    app.update_block(|b| {
        b.time = b.time.plus_seconds(24 * 60 * 60);
    });

    let res = app.execute_contract(Addr::unchecked(USER1), faucet_addr.clone(), &claim_msg, &[]);
    assert!(res.is_ok());

    let new_balance1: cw20::BalanceResponse = app.wrap().query_wasm_smart(
        &token1_addr,
        &cw20::Cw20QueryMsg::Balance { address: USER1.to_string() },
    ).unwrap();
    assert_eq!(new_balance1.balance, balance1.balance * Uint128::new(2));

    let new_balance2: cw20::BalanceResponse = app.wrap().query_wasm_smart(
        &token2_addr,
        &cw20::Cw20QueryMsg::Balance { address: USER1.to_string() },
    ).unwrap();
    assert_eq!(new_balance2.balance, balance2.balance * Uint128::new(2));

    let new_native_balance = app.wrap().query_balance(USER1, "uusd").unwrap();
    assert_eq!(new_native_balance.amount, native_balance.amount * Uint128::new(2));

    let last_claim: GetLastClaimResponse = app.wrap().query_wasm_smart(
        &faucet_addr,
        &QueryMsg::GetLastClaim { address: USER1.to_string() },
    ).unwrap();
    assert!(last_claim.last_claim_time.is_some());

    let total_claims: GetTotalClaimsResponse = app.wrap().query_wasm_smart(
        &faucet_addr,
        &QueryMsg::GetTotalClaims { address: USER1.to_string() },
    ).unwrap();
    assert_eq!(total_claims.total_claims, Uint128::new(2));
}

#[test]
fn test_withdraw() {
    let mut app = mock_app();
    let (faucet_addr, token1_addr, token2_addr) = setup_test_case(&mut app);

    let faucet_token1_balance: cw20::BalanceResponse = app.wrap().query_wasm_smart(
        &token1_addr,
        &cw20::Cw20QueryMsg::Balance { address: faucet_addr.to_string() },
    ).unwrap();
    let faucet_token2_balance: cw20::BalanceResponse = app.wrap().query_wasm_smart(
        &token2_addr,
        &cw20::Cw20QueryMsg::Balance { address: faucet_addr.to_string() },
    ).unwrap();
    let faucet_native_balance = app.wrap().query_balance(&faucet_addr, "uusd").unwrap();

    let initial_owner_token1_balance: cw20::BalanceResponse = app.wrap().query_wasm_smart(
        &token1_addr,
        &cw20::Cw20QueryMsg::Balance { address: OWNER.to_string() },
    ).unwrap();
    let initial_owner_token2_balance: cw20::BalanceResponse = app.wrap().query_wasm_smart(
        &token2_addr,
        &cw20::Cw20QueryMsg::Balance { address: OWNER.to_string() },
    ).unwrap();
    let initial_owner_native_balance = app.wrap().query_balance(OWNER, "uusd").unwrap();

    let withdraw_msg = ExecuteMsg::Withdraw {};
    let res = app.execute_contract(Addr::unchecked(OWNER), faucet_addr.clone(), &withdraw_msg, &[]);
    assert!(res.is_ok());

    let owner_token1_balance: cw20::BalanceResponse = app.wrap().query_wasm_smart(
        &token1_addr,
        &cw20::Cw20QueryMsg::Balance { address: OWNER.to_string() },
    ).unwrap();
    let owner_token2_balance: cw20::BalanceResponse = app.wrap().query_wasm_smart(
        &token2_addr,
        &cw20::Cw20QueryMsg::Balance { address: OWNER.to_string() },
    ).unwrap();
    let owner_native_balance = app.wrap().query_balance(OWNER, "uusd").unwrap();

    assert_eq!(owner_token1_balance.balance, initial_owner_token1_balance.balance + faucet_token1_balance.balance);
    assert_eq!(owner_token2_balance.balance, initial_owner_token2_balance.balance + faucet_token2_balance.balance);
    assert_eq!(owner_native_balance.amount, initial_owner_native_balance.amount + faucet_native_balance.amount);

    let final_faucet_token1_balance: cw20::BalanceResponse = app.wrap().query_wasm_smart(
        &token1_addr,
        &cw20::Cw20QueryMsg::Balance { address: faucet_addr.to_string() },
    ).unwrap();
    let final_faucet_token2_balance: cw20::BalanceResponse = app.wrap().query_wasm_smart(
        &token2_addr,
        &cw20::Cw20QueryMsg::Balance { address: faucet_addr.to_string() },
    ).unwrap();
    let final_faucet_native_balance = app.wrap().query_balance(&faucet_addr, "uusd").unwrap();

    assert_eq!(final_faucet_token1_balance.balance, Uint128::zero());
    assert_eq!(final_faucet_token2_balance.balance, Uint128::zero());
    assert_eq!(final_faucet_native_balance.amount, Uint128::zero());

    let err = app.execute_contract(Addr::unchecked(USER1), faucet_addr.clone(), &withdraw_msg, &[]).unwrap_err();
    assert!(err.root_cause().to_string().contains("Caller is not the contract's current owner"));
}