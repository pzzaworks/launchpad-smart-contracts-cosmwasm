use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{from_json, Addr, coins, BankMsg, Env, MessageInfo, OwnedDeps, Timestamp, Uint128};
use crate::msg::{ClaimableAmountResponse, ConfigResponse, ExecuteMsg, InstantiateMsg, NextUnlockDateResponse, QueryMsg, VestedAmountResponse, WhitelistInfoResponse};
use crate::contract::{instantiate, execute, query};
use crate::error::ContractError;

const OWNER: &str = "owner";
const USER1: &str = "user1";
const USER2: &str = "user2";
const DENOM: &str = "utoken";
const DECIMALS: u32 = 6;
const SCALE: u128 = 10u128.pow(DECIMALS);

fn mock_instantiate() -> (OwnedDeps<MockStorage, MockApi, MockQuerier>, Env, MessageInfo) {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(OWNER, &[]);
    
    let msg = InstantiateMsg {
        owner: Some(OWNER.to_string()),
        denom: DENOM.to_string(),
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
    assert_eq!(config.config.denom, DENOM);
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
        denom: DENOM.to_string(),
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
        denom: DENOM.to_string(),
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
        denom: DENOM.to_string(),
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
        denom: DENOM.to_string(),
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
            cosmwasm_std::CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                assert_eq!(to_address, USER1);
                assert_eq!(amount, &coins(expected, DENOM), "Time: {}, Expected: {}, Actual: {}", env.block.time.seconds(), expected, amount[0].amount);
                Ok(amount[0].amount)
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
        denom: DENOM.to_string(),
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
        cosmwasm_std::CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
            assert_eq!(to_address, USER1);
            assert_eq!(amount, &coins(5187500000, DENOM));
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