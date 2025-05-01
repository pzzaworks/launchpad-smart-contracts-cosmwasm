use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{attr, from_json, Addr, CosmosMsg, Env, MessageInfo, OwnedDeps, Timestamp, Uint128, WasmMsg};
use cw20::Cw20ExecuteMsg;
use crate::msg::{ClaimableAmountResponse, ConfigResponse, ExecuteMsg, InstantiateMsg, NextUnlockDateResponse, QueryMsg, StatisticsResponse, VestedAmountResponse, WhitelistInfoResponse};
use crate::contract::{instantiate, execute, query};
use crate::error::ContractError;
use crate::state::Config;

const OWNER: &str = "owner";
const USER1: &str = "user1";
const USER2: &str = "user2";
const TOKEN_ADDR: &str = "token_address";
const DECIMALS: u32 = 6;
const SCALE: u128 = 10u128.pow(DECIMALS);

fn mock_instantiate() -> (OwnedDeps<MockStorage, MockApi, MockQuerier>, Env, MessageInfo) {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(OWNER, &[]);
    
    let msg = InstantiateMsg {
        owner: Some(OWNER.to_string()),
        token: TOKEN_ADDR.to_string(),
        fee_address: OWNER.to_string(),
        total_token_on_sale: Uint128::new(1_000_000 * SCALE),
        grace_period: 86400,
        platform_fee: Uint128::new(100),
        decimals: 6,
        start: 1625097600,
        cliff: 1625097600 + 2592000,
        duration: 2592000, 
        initial_unlock_percent: 1750, 
        linear_vesting_count: 12,
    };

    let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
    assert_eq!(0, res.messages.len());

    (deps, env, info)
}

#[test]
fn proper_initialization() {
    let (deps, env, _info) = mock_instantiate();

    let res = query(deps.as_ref(), env, QueryMsg::GetConfig {}).unwrap();
    let config: ConfigResponse = from_json(&res).unwrap();
    assert_eq!(config.config.token, TOKEN_ADDR);
    assert_eq!(config.config.fee_address, Addr::unchecked(OWNER));
    assert_eq!(config.config.total_token_on_sale, Uint128::new(1_000_000 * SCALE));
}

#[test]
fn test_set_whitelist() {
    let (mut deps, env, info) = mock_instantiate();

    let msg = ExecuteMsg::SetWhitelist {
        tag_id: "tag1".to_string(),
        wallets: vec![USER1.to_string(), USER2.to_string()],
        payment_amounts: vec![Uint128::new(1000 * SCALE), Uint128::new(2000 * SCALE)],
        token: TOKEN_ADDR.to_string(),
        token_amounts: vec![Uint128::new(10_000 * SCALE), Uint128::new(20_000 * SCALE)],
        refund_fee: Uint128::new(50),
    };

    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(3, res.attributes.len()); 

    let res = query(
        deps.as_ref(),
        env,
        QueryMsg::GetWhitelistInfo {
            wallet: USER1.to_string(),
        },
    )
    .unwrap();
    let whitelist_info: WhitelistInfoResponse = from_json(&res).unwrap();

    assert_eq!(whitelist_info.info.amount, Uint128::new(10_000 * SCALE));
}

#[test]
fn test_vesting_calculation() {
    let (mut deps, mut env, info) = mock_instantiate();

    let msg = ExecuteMsg::SetWhitelist {
        tag_id: "tag1".to_string(),
        wallets: vec![USER1.to_string()],
        payment_amounts: vec![Uint128::new(1000 * SCALE)],
        token: TOKEN_ADDR.to_string(),
        token_amounts: vec![Uint128::new(10_000 * SCALE)],
        refund_fee: Uint128::new(50),
    };

    execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    let check_vested_amount = |env: &Env, expected: u128| {
        let res = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::GetVestedAmount {
                wallet: USER1.to_string(),
            },
        )
        .unwrap();
        let vested: VestedAmountResponse = from_json(&res).unwrap();
        assert_eq!(vested.amount, Uint128::new(expected), "Time: {}", env.block.time.seconds());
    };

    env.block.time = Timestamp::from_seconds(1625097600); // start
    check_vested_amount(&env, 1750000000); 

    env.block.time = Timestamp::from_seconds(1627689600); // cliff (1 month after start)
    check_vested_amount(&env, 1750000000); 

    env.block.time = Timestamp::from_seconds(1627689601); // Just after cliff
    check_vested_amount(&env, 1750000000); 
    
    env.block.time = Timestamp::from_seconds(1630281600); // 2 months after start (1 month after cliff)
    check_vested_amount(&env, 2437500000); 
    
    env.block.time = Timestamp::from_seconds(1640865600); // ~5 months after cliff
    check_vested_amount(&env, 5187500000); 

    env.block.time = Timestamp::from_seconds(1658793600); // vesting end
    check_vested_amount(&env, 10000000000); // Fully vested
}

#[test]
fn test_vesting_with_cliff() {
    let (mut deps, mut env, info) = mock_instantiate();

    let msg = ExecuteMsg::SetWhitelist {
        tag_id: "tag1".to_string(),
        wallets: vec![USER1.to_string()],
        payment_amounts: vec![Uint128::new(1000 * SCALE)],
        token: TOKEN_ADDR.to_string(),
        token_amounts: vec![Uint128::new(10_000 * SCALE)],
        refund_fee: Uint128::new(50),
    };

    execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    let check_amounts = |env: &Env, expected_vested: u128, expected_claimable: u128| {
        let res = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::GetVestedAmount {
                wallet: USER1.to_string(),
            },
        )
        .unwrap();
        let vested: VestedAmountResponse = from_json(&res).unwrap();
        assert_eq!(vested.amount, Uint128::new(expected_vested), "Time: {}, Vested", env.block.time.seconds());

        let res = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::GetClaimableAmount {
                wallet: USER1.to_string(),
            },
        )
        .unwrap();
        let claimable: ClaimableAmountResponse = from_json(&res).unwrap();
        assert_eq!(claimable.amount, Uint128::new(expected_claimable), "Time: {}, Claimable", env.block.time.seconds());
    };

    env.block.time = Timestamp::from_seconds(1625097600); 
    check_amounts(&env, 1750000000, 1750000000);

    env.block.time = Timestamp::from_seconds(1627689600); // 1 month after start (cliff)
    check_amounts(&env, 1750000000, 1750000000);

    env.block.time = Timestamp::from_seconds(1630281600); // 2 months after start (1 month after cliff)
    check_amounts(&env, 2437500000, 2437500000);

    env.block.time = Timestamp::from_seconds(1640865600); // ~5 months after cliff
    check_amounts(&env, 5187500000, 5187500000);

    env.block.time = Timestamp::from_seconds(1658793600); // vesting end
    check_amounts(&env, 10000000000, 10000000000);
}

#[test]
fn test_multiple_vesting_claims() {
    let (mut deps, mut env, info) = mock_instantiate();

    let msg = ExecuteMsg::SetWhitelist {
        tag_id: "tag1".to_string(),
        wallets: vec![USER1.to_string()],
        payment_amounts: vec![Uint128::new(1000 * SCALE)],
        token: TOKEN_ADDR.to_string(),
        token_amounts: vec![Uint128::new(10_000 * SCALE)],
        refund_fee: Uint128::new(50),
    };

    execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    let mut total_claimed = Uint128::zero();

    let mut claim_tokens = |env: &Env, expected: u128| -> Result<Uint128, ContractError> {
        let msg = ExecuteMsg::ClaimVestedTokens {};
        let info = mock_info(USER1, &[]);
        let res = execute(deps.as_mut(), env.clone(), info, msg)?;

        if res.messages.is_empty() {
            return Ok(Uint128::zero());
        }

        match &res.messages[0].msg {
            CosmosMsg::Wasm(WasmMsg::Execute { contract_addr, msg, .. }) => {
                assert_eq!(contract_addr, TOKEN_ADDR);
                match from_json(msg).unwrap() {
                    Cw20ExecuteMsg::Transfer { recipient, amount } => {
                        assert_eq!(recipient, USER1);
                        assert_eq!(amount, Uint128::new(expected), "Time: {}, Expected: {}, Actual: {}", env.block.time.seconds(), expected, amount);
                        Ok(amount)
                    },
                    _ => panic!("Unexpected execute message"),
                }
            },
            _ => panic!("Unexpected message type"),
        }
    };

    env.block.time = Timestamp::from_seconds(1627689600); // 1 month after start (cliff)
    let claimed = claim_tokens(&env, 1750000000).unwrap();
    total_claimed += claimed;

    env.block.time = Timestamp::from_seconds(1630281600); // 2 months after start
    let claimed = claim_tokens(&env, 687500000).unwrap(); // 2437500000 - 1750000000
    total_claimed += claimed;

    env.block.time = Timestamp::from_seconds(1640865600); // ~5 months after cliff
    let claimed = claim_tokens(&env, 2750000000).unwrap(); // 5187500000 - 2437500000
    total_claimed += claimed;

    env.block.time = Timestamp::from_seconds(1658793600); // vesting end
    let claimed = claim_tokens(&env, 4812500000).unwrap(); // 10000000000 - 5187500000
    total_claimed += claimed;

    assert_eq!(total_claimed, Uint128::new(10000000000), "Total claimed: {}", total_claimed);

    let result = claim_tokens(&env, 0);
    assert!(matches!(result, Err(ContractError::NoTokensToClaim {})));
}

#[test]
fn test_claim_vested_tokens() {
    let (mut deps, mut env, info) = mock_instantiate();

    let msg = ExecuteMsg::SetWhitelist {
        tag_id: "tag1".to_string(),
        wallets: vec![USER1.to_string()],
        payment_amounts: vec![Uint128::new(1000 * SCALE)],
        token: TOKEN_ADDR.to_string(),
        token_amounts: vec![Uint128::new(10_000 * SCALE)],
        refund_fee: Uint128::new(50),
    };

    execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    env.block.time = Timestamp::from_seconds(1640865600); // ~5 months after cliff

    let msg = ExecuteMsg::ClaimVestedTokens {};
    let info = mock_info(USER1, &[]);

    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    assert_eq!(1, res.messages.len());
    match &res.messages[0].msg {
        CosmosMsg::Wasm(WasmMsg::Execute { contract_addr, msg, .. }) => {
            assert_eq!(contract_addr, TOKEN_ADDR);
            match from_json(msg).unwrap() {
                Cw20ExecuteMsg::Transfer { recipient, amount } => {
                    assert_eq!(recipient, USER1);
                    assert_eq!(amount, Uint128::new(5187500000), "Expected: 5187500000, Actual: {}", amount);
                },
                _ => panic!("Unexpected execute message"),
            }
        },
        _ => panic!("Unexpected message type"),
    }
}

#[test]
fn test_next_unlock_date() {
    let (deps, mut env, _info) = mock_instantiate();

    env.block.time = Timestamp::from_seconds(1625097600); // start
    let res = query(deps.as_ref(), env.clone(), QueryMsg::GetNextUnlockDate {}).unwrap();
    let next_unlock: NextUnlockDateResponse = from_json(&res).unwrap();
    assert_eq!(next_unlock.next_unlock_date, 1627689600); // 1 month after start (cliff)

    env.block.time = Timestamp::from_seconds(1627689600); // 1 month after start (cliff)
    let res = query(deps.as_ref(), env.clone(), QueryMsg::GetNextUnlockDate {}).unwrap();
    let next_unlock: NextUnlockDateResponse = from_json(&res).unwrap();
    assert_eq!(next_unlock.next_unlock_date, 1630281600); // 2 months after start

    env.block.time = Timestamp::from_seconds(1630281600); // 2 months after start
    let res = query(deps.as_ref(), env.clone(), QueryMsg::GetNextUnlockDate {}).unwrap();
    let next_unlock: NextUnlockDateResponse = from_json(&res).unwrap();
    assert_eq!(next_unlock.next_unlock_date, 1632873600); // 3 months after start

    env.block.time = Timestamp::from_seconds(1658793600); // vesting end
    let res = query(deps.as_ref(), env.clone(), QueryMsg::GetNextUnlockDate {}).unwrap();
    let next_unlock: NextUnlockDateResponse = from_json(&res).unwrap();
    assert_eq!(next_unlock.next_unlock_date, 1658793600); // vesting end
}

#[test]
fn test_add_to_whitelist() {
    let (mut deps, env, info) = mock_instantiate();

    let msg = ExecuteMsg::AddToWhitelist {
        tag_id: "tag1".to_string(),
        wallets: vec![USER1.to_string()],
        payment_amounts: vec![Uint128::new(1000 * SCALE)],
        token: TOKEN_ADDR.to_string(),
        token_amounts: vec![Uint128::new(10_000 * SCALE)],
        refund_fee: Uint128::new(50),
    };

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
    assert_eq!(3, res.attributes.len());

    let msg = ExecuteMsg::AddToWhitelist {
        tag_id: "tag1".to_string(),
        wallets: vec![USER2.to_string()],
        payment_amounts: vec![Uint128::new(2000 * SCALE)],
        token: TOKEN_ADDR.to_string(),
        token_amounts: vec![Uint128::new(20_000 * SCALE)],
        refund_fee: Uint128::new(50),
    };

    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(3, res.attributes.len());

    let res = query(
        deps.as_ref(),
        env.clone(),
        QueryMsg::GetWhitelistInfo {
            wallet: USER1.to_string(),
        },
    )
    .unwrap();
    let whitelist_info: WhitelistInfoResponse = from_json(&res).unwrap();
    assert_eq!(whitelist_info.info.amount, Uint128::new(10_000 * SCALE));

    let res = query(
        deps.as_ref(),
        env,
        QueryMsg::GetWhitelistInfo {
            wallet: USER2.to_string(),
        },
    )
    .unwrap();
    let whitelist_info: WhitelistInfoResponse = from_json(&res).unwrap();
    assert_eq!(whitelist_info.info.amount, Uint128::new(20_000 * SCALE));

    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetStatistics {}).unwrap();
    let statistics: StatisticsResponse = from_json(&res).unwrap();
    assert_eq!(statistics.statistics.total_vested_token, Uint128::new(30_000 * SCALE));
}

#[test]
fn test_vesting_over_linear_periods() {
    let (mut deps, mut env, info) = mock_instantiate();

    let msg = ExecuteMsg::SetWhitelist {
        tag_id: "tag1".to_string(),
        wallets: vec![USER1.to_string()],
        payment_amounts: vec![Uint128::new(1000 * SCALE)],
        token: TOKEN_ADDR.to_string(),
        token_amounts: vec![Uint128::new(10_000 * SCALE)],
        refund_fee: Uint128::new(50),
    };

    execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    let config = query(deps.as_ref(), env.clone(), QueryMsg::GetConfig {}).unwrap();
    let config: ConfigResponse = from_json(&config).unwrap();

    let total_amount = Uint128::new(10_000 * SCALE);
    let initial_unlock_percent = Uint128::new(config.config.initial_unlock_percent as u128);
    let initial_unlock = total_amount * initial_unlock_percent / Uint128::new(10000);
    let remaining = total_amount - initial_unlock;
    let vesting_duration = config.config.duration;
    let linear_vesting_count = config.config.linear_vesting_count as u64;

    let mut total_claimed = Uint128::zero();

    let calculate_expected_vested = |periods_passed: u64| -> Uint128 {
        let vesting_start = config.config.cliff.max(config.config.start);
        let vesting_end = vesting_start + (vesting_duration * linear_vesting_count);
        let current_time = config.config.start + periods_passed * vesting_duration;
    
        if current_time >= vesting_end {
            total_amount
        } else if current_time <= vesting_start {
            initial_unlock
        } else {
            let time_passed = current_time - vesting_start;
            let total_vesting_time = vesting_end - vesting_start;
            let steps_passed = periods_passed - 1;  
    
            if steps_passed == linear_vesting_count - 1 {
                total_amount
            } else {
                let vested_remaining = remaining * Uint128::from(time_passed) / Uint128::from(total_vesting_time);
                let vested_amount = initial_unlock + vested_remaining;
                std::cmp::min(vested_amount, total_amount)
            }
        }
    };

    for i in 0..=linear_vesting_count {
        env.block.time = Timestamp::from_seconds(config.config.start + i * vesting_duration);

        let expected_vested = calculate_expected_vested(i);

        let vested_res = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::GetVestedAmount { wallet: USER1.to_string() },
        ).unwrap();
        let vested: VestedAmountResponse = from_json(&vested_res).unwrap();
        assert_eq!(vested.amount, expected_vested, "Period {}: Incorrect vested amount", i);

        let claimable_res = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::GetClaimableAmount { wallet: USER1.to_string() },
        ).unwrap();
        let claimable: ClaimableAmountResponse = from_json(&claimable_res).unwrap();

        let expected_claimable = expected_vested - total_claimed;
        assert_eq!(claimable.amount, expected_claimable, "Period {}: Incorrect claimable amount", i);

        if !expected_claimable.is_zero() {
            let msg = ExecuteMsg::ClaimVestedTokens {};
            let info = mock_info(USER1, &[]);
            let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

            assert_eq!(1, res.messages.len());
            match &res.messages[0].msg {
                CosmosMsg::Wasm(WasmMsg::Execute { contract_addr, msg, .. }) => {
                    assert_eq!(contract_addr, TOKEN_ADDR);
                    match from_json(msg).unwrap() {
                        Cw20ExecuteMsg::Transfer { recipient, amount } => {
                            assert_eq!(recipient, USER1);
                            assert_eq!(amount, expected_claimable, "Period {}: Expected claim: {}, Actual: {}", i, expected_claimable, amount);
                            total_claimed += amount;
                        },
                        _ => panic!("Unexpected execute message"),
                    }
                },
                _ => panic!("Unexpected message type"),
            }
        }

        println!("Period {}: Vested = {}, Claimed = {}, Total Claimed = {}", 
                 i, expected_vested, expected_claimable, total_claimed);
    }

    let final_vested = calculate_expected_vested(linear_vesting_count);
    assert_eq!(final_vested, total_amount, "Final vested amount should equal total amount");

    assert_eq!(total_claimed, total_amount, "Total claimed: {}, Expected: {}", total_claimed, total_amount);

    let msg = ExecuteMsg::ClaimVestedTokens {};
    let info = mock_info(USER1, &[]);
    let result = execute(deps.as_mut(), env, info, msg);
    assert!(matches!(result, Err(ContractError::NoTokensToClaim {})));
}
#[test]
fn test_update_config() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(OWNER, &[]);

    let msg = InstantiateMsg {
        owner: Some(OWNER.to_string()),
        token: TOKEN_ADDR.to_string(),
        fee_address: OWNER.to_string(),
        total_token_on_sale: Uint128::new(1_000_000 * SCALE),
        grace_period: 86400,
        platform_fee: Uint128::new(100),
        decimals: 6,
        start: 1625097600,
        cliff: 1625097600 + 2592000,
        duration: 2592000,
        initial_unlock_percent: 1750,
        linear_vesting_count: 12,
    };

    let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    let new_config = Config {
        token: Addr::unchecked("new_token_address"),
        fee_address: Addr::unchecked("new_fee_address"),
        total_token_on_sale: Uint128::new(2_000_000 * SCALE),
        grace_period: 172800,
        platform_fee: Uint128::new(200),
        decimals: 8,
        start: 1625184000,
        cliff: 1625184000 + 5184000,
        duration: 5184000,
        initial_unlock_percent: 2000,
        linear_vesting_count: 24,
    };

    let update_msg = ExecuteMsg::UpdateConfig(new_config.clone());
    let res = execute(deps.as_mut(), env.clone(), info, update_msg).unwrap();

    assert!(res.attributes.len() > 0, "Response should have attributes");
    assert_eq!(res.attributes[0], attr("action", "update_config"));

    for attr in res.attributes.iter().skip(1) {
        match attr.key.as_str() {
            "token" => assert_eq!(attr.value, new_config.token.to_string()),
            "fee_address" => assert_eq!(attr.value, new_config.fee_address.to_string()),
            "total_token_on_sale" => assert_eq!(attr.value, new_config.total_token_on_sale.to_string()),
            "grace_period" => assert_eq!(attr.value, new_config.grace_period.to_string()),
            "platform_fee" => assert_eq!(attr.value, new_config.platform_fee.to_string()),
            "decimals" => assert_eq!(attr.value, new_config.decimals.to_string()),
            "start" => assert_eq!(attr.value, new_config.start.to_string()),
            "cliff" => assert_eq!(attr.value, new_config.cliff.to_string()),
            "duration" => assert_eq!(attr.value, new_config.duration.to_string()),
            "initial_unlock_percent" => assert_eq!(attr.value, new_config.initial_unlock_percent.to_string()),
            "linear_vesting_count" => assert_eq!(attr.value, new_config.linear_vesting_count.to_string()),
            _ => panic!("Unexpected attribute: {}", attr.key),
        }
    }

    let query_msg = QueryMsg::GetConfig {};
    let res = query(deps.as_ref(), env, query_msg).unwrap();
    let updated_config: ConfigResponse = from_json(&res).unwrap();

    assert_eq!(updated_config.config.token, new_config.token);
    assert_eq!(updated_config.config.fee_address, new_config.fee_address);
    assert_eq!(updated_config.config.total_token_on_sale, new_config.total_token_on_sale);
    assert_eq!(updated_config.config.grace_period, new_config.grace_period);
    assert_eq!(updated_config.config.platform_fee, new_config.platform_fee);
    assert_eq!(updated_config.config.decimals, new_config.decimals);
    assert_eq!(updated_config.config.start, new_config.start);
    assert_eq!(updated_config.config.cliff, new_config.cliff);
    assert_eq!(updated_config.config.duration, new_config.duration);
    assert_eq!(updated_config.config.initial_unlock_percent, new_config.initial_unlock_percent);
    assert_eq!(updated_config.config.linear_vesting_count, new_config.linear_vesting_count);
}