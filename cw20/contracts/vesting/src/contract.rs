#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, to_json_binary, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult, Uint128, Uint64, WasmMsg
};
use cw20::Cw20ExecuteMsg;
use crate::hooks::{whitelist_hook_msgs, claim_hook_msgs, refund_hook_msgs};
use crate::msg::{
    ClaimableAmountResponse, ConfigResponse, ExecuteMsg, GetHooksResponse, InstantiateMsg, IsWhitelistedResponse, NextUnlockDateResponse, PaymentTokenResponse, QueryMsg, StatisticsResponse, UserInfoResponse, VestedAmountResponse, VestingDetailsResponse, WhitelistBatchResponse, WhitelistIndexResponse, WhitelistInfoResponse
};
use crate::state::{
    Config, Statistics, UserInfo, WhitelistInfo, CONFIG, HOOKS, IS_WHITELISTED, PAYMENT_TOKEN, STATISTICS, USER_INFO, WHITELIST_INDEX, WHITELIST_POOL
};
use crate::ContractError;
use cw2::set_contract_version;
use std::cmp::min;

pub(crate) const CONTRACT_NAME: &str = "crates.io:stake";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    cw_ownable::initialize_owner(deps.storage, deps.api, msg.owner.as_deref())?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        token: deps.api.addr_validate(&msg.token)?,
        fee_address: deps.api.addr_validate(&msg.fee_address)?,
        total_token_on_sale: msg.total_token_on_sale,
        grace_period: msg.grace_period,
        platform_fee: msg.platform_fee,
        decimals: msg.decimals,
        start: msg.start,
        cliff: msg.cliff,
        duration: msg.duration,
        initial_unlock_percent: msg.initial_unlock_percent,
        linear_vesting_count: msg.linear_vesting_count,
    };

    CONFIG.save(deps.storage, &config)?;

    let statistics = Statistics {
        total_vested_token: Uint128::zero(),
        total_returned_token: Uint128::zero(),
    };

    STATISTICS.save(deps.storage, &statistics, env.block.height)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender)
        .add_attribute("token", msg.token)
        .add_attribute("total_tokens", msg.total_token_on_sale)
        .add_attribute("start_time", msg.start.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::SetVestingStart { new_start } => execute_set_vesting_start(deps, env, info, new_start),
        ExecuteMsg::UpdateToken { new_token } => execute_update_token(deps, env, info, new_token),
        ExecuteMsg::RequestRefund { tag_id } => execute_request_refund(deps, env, info, tag_id),
        ExecuteMsg::ClaimFunds { token } => execute_claim_funds(deps, env, info, token),
        ExecuteMsg::ClaimVestedTokens {} => execute_claim_vested_tokens(deps, env, info),
        ExecuteMsg::SetWhitelist { tag_id, wallets, payment_amounts, token, token_amounts, refund_fee } => 
            execute_set_whitelist(deps, env, info, tag_id, wallets, payment_amounts, token, token_amounts, refund_fee),
        ExecuteMsg::AddToWhitelist { tag_id, wallets, payment_amounts, token, token_amounts, refund_fee } => 
            execute_add_to_whitelist(deps, env, info, tag_id, wallets, payment_amounts, token, token_amounts, refund_fee),
        ExecuteMsg::EmergencyWithdraw { tag_id } => execute_emergency_withdraw(deps, env, info, tag_id),
        ExecuteMsg::AddHook { addr } => execute_add_hook(deps, env, info, addr),
        ExecuteMsg::RemoveHook { addr } => execute_remove_hook(deps, env, info, addr),
        ExecuteMsg::UpdateOwnership(action) => execute_update_owner(deps, info, env, action),
        ExecuteMsg::UpdateConfig(new_config) => execute_update_config(deps, env, info, new_config),
    }
}

pub fn execute_set_vesting_start(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_start: u64,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
    let mut config = CONFIG.load(deps.storage)?;
    let cliff_duration = config.cliff - config.start;
    config.start = new_start;
    config.cliff = new_start + cliff_duration;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attribute("action", "set_vesting_start"))
}

pub fn execute_update_token(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    new_token: String,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
    let mut config = CONFIG.load(deps.storage)?;
    if env.block.time.seconds() >= config.start {
        return Err(ContractError::VestingAlreadyStarted {});
    }
    config.token = deps.api.addr_validate(&new_token)?;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new()
        .add_attribute("action", "update_token")
        .add_attribute("new_token", new_token))
}

pub fn execute_request_refund(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    tag_id: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut user_info = USER_INFO.load(deps.storage, (&tag_id, &info.sender))?;
    let mut whitelist = WHITELIST_POOL.load(deps.storage, &info.sender)?;

    if env.block.time.seconds() < config.start || env.block.time.seconds() >= config.start + config.grace_period {
        return Err(ContractError::NotInGracePeriod {});
    }
    if user_info.refunded {
        return Err(ContractError::UserAlreadyRefunded {});
    }
    if whitelist.distributed_amount != Uint128::zero() {
        return Err(ContractError::UserAlreadyClaimed {});
    }

    let fee = user_info.payment_amount * user_info.refund_fee / Uint128::new(10u128.pow(config.decimals as u32));
    let refund_amount = user_info.payment_amount - fee;

    user_info.refunded = true;
    user_info.refund_date = Some(env.block.time.seconds());
    USER_INFO.save(deps.storage, (&tag_id, &info.sender), &user_info, env.block.height)?;

    let payment_token = PAYMENT_TOKEN.load(deps.storage, &tag_id)?;

    let mut statistics = STATISTICS.load(deps.storage)?;
    statistics.total_returned_token += user_info.token_amount;
    STATISTICS.save(deps.storage, &statistics, env.block.height)?;

    whitelist.amount -= user_info.token_amount;
    WHITELIST_POOL.save(deps.storage, &info.sender, &whitelist, env.block.height)?;

    let refund_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![coin(refund_amount.u128(), payment_token.to_string())],
    });

    let fee_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: config.fee_address.to_string(),
        amount: vec![coin(fee.u128(), payment_token.to_string())],
    });

    let hook_msgs = refund_hook_msgs(deps.storage, info.sender.clone(), refund_amount)?;

    Ok(Response::new()
        .add_message(refund_msg)
        .add_message(fee_msg)
        .add_submessages(hook_msgs)
        .add_attribute("action", "refund")
        .add_attribute("amount", refund_amount.to_string()))
}

pub fn execute_claim_funds(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token: String,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
    let config = CONFIG.load(deps.storage)?;
    let token_addr = deps.api.addr_validate(&token)?;
    if token_addr == config.token {
        return Err(ContractError::InvalidToken {});
    }

    let mut statistics = STATISTICS.load(deps.storage)?;
    let claimable_amount = statistics.total_vested_token - statistics.total_returned_token;

    if claimable_amount > Uint128::zero() {
        let platform_fee = claimable_amount * config.platform_fee / Uint128::new(10u128.pow(config.decimals as u32));
        let final_claimable_amount = claimable_amount - platform_fee;

        let mut messages = vec![];

        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: token_addr.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: info.sender.to_string(),
                amount: final_claimable_amount,
            })?,
            funds: vec![],
        }));

        if platform_fee > Uint128::zero() {
            messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: token_addr.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: config.fee_address.to_string(),
                    amount: platform_fee,
                })?,
                funds: vec![],
            }));
        }

        statistics.total_vested_token -= claimable_amount;
        STATISTICS.save(deps.storage, &statistics, env.block.height)?;

        Ok(Response::new()
            .add_messages(messages)
            .add_attribute("action", "claim_funds")
            .add_attribute("amount", final_claimable_amount.to_string())
            .add_attribute("platform_fee", platform_fee.to_string()))
    } else {
        Err(ContractError::NoFundsToClaim {})
    }
}

pub fn execute_claim_vested_tokens(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut whitelist = WHITELIST_POOL.load(deps.storage, &info.sender)?;

    let vested_amount = calculate_vested_amount(&config, &whitelist, env.block.time.seconds())?;
    let claimable_amount = vested_amount.saturating_sub(whitelist.distributed_amount);

    if claimable_amount.is_zero() {
        return Err(ContractError::NoTokensToClaim {});
    }

    whitelist.distributed_amount += claimable_amount;
    WHITELIST_POOL.save(deps.storage, &info.sender, &whitelist, env.block.height)?;

    let transfer_msg = WasmMsg::Execute {
        contract_addr: config.token.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
            recipient: info.sender.to_string(),
            amount: claimable_amount,
        })?,
        funds: vec![],
    };

    let hook_msgs = claim_hook_msgs(deps.storage, info.sender.clone(), claimable_amount)?;

    Ok(Response::new()
        .add_message(transfer_msg)
        .add_submessages(hook_msgs)
        .add_attribute("action", "claim_vested_tokens")
        .add_attribute("amount", claimable_amount.to_string()))
}

pub fn execute_set_whitelist(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    tag_id: String,
    wallets: Vec<String>,
    payment_amounts: Vec<Uint128>,
    token: String,
    token_amounts: Vec<Uint128>,
    refund_fee: Uint128,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    if wallets.len() != payment_amounts.len() || wallets.len() != token_amounts.len() {
        return Err(ContractError::MismatchedArrayLengths {});
    }

    let token_addr = deps.api.addr_validate(&token)?;
    PAYMENT_TOKEN.save(deps.storage, &tag_id, &token_addr, env.block.height)?;

    let mut statistics = STATISTICS.load(deps.storage)?;

    let mut messages = vec![];

    for (i, wallet) in wallets.iter().enumerate() {
        let wallet_addr = deps.api.addr_validate(wallet)?;
        let payment_amount = payment_amounts[i];
        let token_amount = token_amounts[i];

        let mut user_info = USER_INFO.may_load(deps.storage, (&tag_id, &wallet_addr))?.unwrap_or_default();
        user_info.payment_amount += payment_amount;
        user_info.token_amount += token_amount;
        user_info.refund_fee = refund_fee;
        USER_INFO.save(deps.storage, (&tag_id, &wallet_addr), &user_info, env.block.height)?;

        let mut whitelist = WHITELIST_POOL.may_load(deps.storage, &wallet_addr)?.unwrap_or_default();
        whitelist.wallet = wallet_addr.clone();
        whitelist.amount += token_amount;
        whitelist.join_date = env.block.time.seconds();
        WHITELIST_POOL.save(deps.storage, &wallet_addr, &whitelist, env.block.height)?;

        let whitelist_index = WHITELIST_INDEX.may_load(deps.storage, &wallet_addr)?.unwrap_or(0);
        WHITELIST_INDEX.save(deps.storage, &wallet_addr, &(whitelist_index + 1), env.block.height)?;

        IS_WHITELISTED.save(deps.storage, &wallet_addr, &true, env.block.height)?;

        statistics.total_vested_token += token_amount;

        messages.extend(whitelist_hook_msgs(deps.storage, wallet_addr.clone(), token_amount)?);
    }

    STATISTICS.save(deps.storage, &statistics, env.block.height)?;

    Ok(Response::new()
        .add_submessages(messages)
        .add_attribute("action", "set_whitelist")
        .add_attribute("tag_id", tag_id)
        .add_attribute("wallets_count", wallets.len().to_string()))
}

pub fn execute_add_to_whitelist(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    tag_id: String,
    wallets: Vec<String>,
    payment_amounts: Vec<Uint128>,
    token: String,
    token_amounts: Vec<Uint128>,
    refund_fee: Uint128,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    if wallets.len() != payment_amounts.len() || wallets.len() != token_amounts.len() {
        return Err(ContractError::MismatchedArrayLengths {});
    }

    let token_addr = deps.api.addr_validate(&token)?;

    PAYMENT_TOKEN.save(deps.storage, &tag_id, &token_addr, env.block.height)?;

    let mut statistics = STATISTICS.load(deps.storage)?;
    let mut messages = vec![];

    for (i, wallet) in wallets.iter().enumerate() {
        let wallet_addr = deps.api.addr_validate(wallet)?;
        let payment_amount = payment_amounts[i];
        let token_amount = token_amounts[i];

        USER_INFO.update(
            deps.storage,
            (&tag_id, &wallet_addr),
            env.block.height,
            |user_info: Option<UserInfo>| -> StdResult<_> {
                let mut info = user_info.unwrap_or_default();
                info.payment_amount += payment_amount;
                info.token_amount += token_amount;
                info.refund_fee = refund_fee;
                Ok(info)
            },
        )?;
        
        WHITELIST_POOL.update(
            deps.storage,
            &wallet_addr,
            env.block.height,
            |whitelist: Option<WhitelistInfo>| -> StdResult<_> {
                let mut info = whitelist.unwrap_or_default();
                info.wallet = wallet_addr.clone();
                info.amount += token_amount;
                info.join_date = env.block.time.seconds();
                Ok(info)
            },
        )?;
        
        WHITELIST_INDEX.update(
            deps.storage,
            &wallet_addr,
            env.block.height,
            |index: Option<u64>| -> StdResult<_> {
                Ok(index.unwrap_or(0) + 1)
            },
        )?;
        
        IS_WHITELISTED.save(deps.storage, &wallet_addr, &true, env.block.height)?;
        
        STATISTICS.update(
            deps.storage,
            env.block.height,
            |stats: Option<Statistics>| -> StdResult<_> {
                let mut statistics = stats.unwrap_or_default();
                statistics.total_vested_token += token_amount;
                Ok(statistics)
            },
        )?;

        statistics.total_vested_token += token_amount;

        messages.extend(whitelist_hook_msgs(deps.storage, wallet_addr.clone(), token_amount)?);
    }

    STATISTICS.save(deps.storage, &statistics, env.block.height)?;

    Ok(Response::new()
        .add_submessages(messages)
        .add_attribute("action", "add_to_whitelist")
        .add_attribute("tag_id", tag_id)
        .add_attribute("wallets_added", wallets.len().to_string()))
}

pub fn execute_emergency_withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    tag_id: String,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
    let config = CONFIG.load(deps.storage)?;
    let balance = deps.querier.query_all_balances(env.contract.address.clone())?;
    let mut messages = vec![];

    if !balance.is_empty() {
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: balance,
        }));
    }

    let token_balance: cw20::BalanceResponse = deps.querier.query_wasm_smart(
        config.token.clone(),
        &cw20::Cw20QueryMsg::Balance {
            address: env.contract.address.to_string(),
        },
    )?;

    if token_balance.balance > Uint128::zero() {
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: info.sender.to_string(),
                amount: token_balance.balance,
            })?,
            funds: vec![],
        }));
    }

    if let Some(payment_token) = PAYMENT_TOKEN.may_load(deps.storage, &tag_id)? {
        let payment_token_balance: cw20::BalanceResponse = deps.querier.query_wasm_smart(
            payment_token.clone(),
            &cw20::Cw20QueryMsg::Balance {
                address: env.contract.address.to_string(),
            },
        )?;

        if payment_token_balance.balance > Uint128::zero() {
            messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: payment_token.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: info.sender.to_string(),
                    amount: payment_token_balance.balance,
                })?,
                funds: vec![],
            }));
        }
    }

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "emergency_withdraw")
        .add_attribute("tag_id", tag_id))
}

pub fn execute_add_hook(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    addr: String,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
    let hook = deps.api.addr_validate(&addr)?;
    HOOKS.add_hook(deps.storage, hook)?;
    Ok(Response::new()
        .add_attribute("action", "add_hook")
        .add_attribute("hook", addr))
}

pub fn execute_remove_hook(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    addr: String,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
    let hook = deps.api.addr_validate(&addr)?;
    HOOKS.remove_hook(deps.storage, hook)?;
    Ok(Response::new()
        .add_attribute("action", "remove_hook")
        .add_attribute("hook", addr))
}

pub fn execute_update_owner(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    action: cw_ownable::Action,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
    let ownership = cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;
    Ok(Response::default().add_attributes(ownership.into_attributes()))
}

pub fn execute_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_config: Config,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    CONFIG.save(deps.storage, &new_config)?;

    Ok(Response::new()
        .add_attribute("action", "update_config")
        .add_attribute("token", new_config.token.to_string())
        .add_attribute("fee_address", new_config.fee_address.to_string())
        .add_attribute("total_token_on_sale", new_config.total_token_on_sale.to_string())
        .add_attribute("grace_period", new_config.grace_period.to_string())
        .add_attribute("platform_fee", new_config.platform_fee.to_string())
        .add_attribute("decimals", new_config.decimals.to_string())
        .add_attribute("start", new_config.start.to_string())
        .add_attribute("cliff", new_config.cliff.to_string())
        .add_attribute("duration", new_config.duration.to_string())
        .add_attribute("initial_unlock_percent", new_config.initial_unlock_percent.to_string())
        .add_attribute("linear_vesting_count", new_config.linear_vesting_count.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    let res = match msg {
        QueryMsg::GetConfig {} => to_json_binary(&query_config(deps)?),
        QueryMsg::GetStatistics {} => to_json_binary(&query_statistics(deps)?),
        QueryMsg::GetWhitelistInfo { wallet } => to_json_binary(&query_whitelist_info(deps, wallet)?),
        QueryMsg::GetVestedAmount { wallet } => to_json_binary(&query_vested_amount(deps, env, wallet)?),
        QueryMsg::GetClaimableAmount { wallet } => to_json_binary(&query_claimable_amount(deps, env, wallet)?),
        QueryMsg::GetWhitelistBatch { start, limit } => to_json_binary(&query_whitelist_batch(deps, start, limit)?),
        QueryMsg::GetVestingDetails {} => to_json_binary(&query_vesting_details(deps)?),
        QueryMsg::IsWhitelisted { wallet } => to_json_binary(&query_is_whitelisted(deps, wallet)?),
        QueryMsg::GetWhitelistIndex { wallet } => to_json_binary(&query_whitelist_index(deps, wallet)?),
        QueryMsg::GetUserInfo { tag_id, wallet } => to_json_binary(&query_user_info(deps, tag_id, wallet)?),
        QueryMsg::GetPaymentToken { tag_id } => to_json_binary(&query_payment_token(deps, tag_id)?),
        QueryMsg::GetNextUnlockDate {} => to_json_binary(&query_next_unlock_date(deps, env)?),
        QueryMsg::GetHooks {} => to_json_binary(&query_hooks(deps)?),
        QueryMsg::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
    };

    res.map_err(|e| ContractError::QueryError { msg: e.to_string() })
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse { config })
}

pub fn query_hooks(deps: Deps) -> StdResult<GetHooksResponse> {
    Ok(GetHooksResponse {
        hooks: HOOKS.query_hooks(deps)?.hooks,
    })
}

pub fn query_statistics(deps: Deps) -> StdResult<StatisticsResponse> {
    let statistics = STATISTICS.load(deps.storage)?;
    Ok(StatisticsResponse { statistics })
}

pub fn query_whitelist_info(deps: Deps, wallet: String) -> StdResult<WhitelistInfoResponse> {
    let wallet_addr = deps.api.addr_validate(&wallet)?;
    let info = WHITELIST_POOL.load(deps.storage, &wallet_addr)?;
    Ok(WhitelistInfoResponse { info })
}

pub fn query_vested_amount(deps: Deps, env: Env, wallet: String) -> StdResult<VestedAmountResponse> {
    let wallet_addr = deps.api.addr_validate(&wallet)?;
    let config = CONFIG.load(deps.storage)?;
    let whitelist = WHITELIST_POOL.load(deps.storage, &wallet_addr)?;
    let amount = calculate_vested_amount(&config, &whitelist, env.block.time.seconds())?;
    Ok(VestedAmountResponse { amount })
}

pub fn query_claimable_amount(deps: Deps, env: Env, wallet: String) -> StdResult<ClaimableAmountResponse> {
    let wallet_addr = deps.api.addr_validate(&wallet)?;
    let config = CONFIG.load(deps.storage)?;
    let whitelist = WHITELIST_POOL.load(deps.storage, &wallet_addr)?;
    let amount = calculate_claimable_amount(&config, &whitelist, env.block.time.seconds())?;
    Ok(ClaimableAmountResponse { amount })
}

pub fn query_whitelist_batch(deps: Deps, start: u64, limit: u32) -> StdResult<WhitelistBatchResponse> {
    let whitelist: StdResult<Vec<_>> = WHITELIST_POOL
        .range(deps.storage, None, None, Order::Ascending)
        .skip(start as usize)
        .take(limit as usize)
        .collect();
    Ok(WhitelistBatchResponse { whitelist: whitelist? })
}

pub fn query_vesting_details(deps: Deps) -> StdResult<VestingDetailsResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(VestingDetailsResponse {
        start: config.start,
        cliff: config.cliff,
        duration: config.duration,
        initial_unlock_percent: config.initial_unlock_percent,
        linear_vesting_count: config.linear_vesting_count,
    })
}

pub fn query_is_whitelisted(deps: Deps, wallet: String) -> StdResult<IsWhitelistedResponse> {
    let wallet_addr = deps.api.addr_validate(&wallet)?;
    let is_whitelisted = IS_WHITELISTED.may_load(deps.storage, &wallet_addr)?.unwrap_or(false);
    Ok(IsWhitelistedResponse { is_whitelisted })
}

pub fn query_whitelist_index(deps: Deps, wallet: String) -> StdResult<WhitelistIndexResponse> {
    let wallet_addr = deps.api.addr_validate(&wallet)?;
    let index = WHITELIST_INDEX.may_load(deps.storage, &wallet_addr)?.unwrap_or(0);
    Ok(WhitelistIndexResponse { index })
}

pub fn query_user_info(deps: Deps, tag_id: String, wallet: String) -> StdResult<UserInfoResponse> {
    let wallet_addr = deps.api.addr_validate(&wallet)?;
    let info = USER_INFO.load(deps.storage, (&tag_id, &wallet_addr))?;
    Ok(UserInfoResponse { info })
}

pub fn query_payment_token(deps: Deps, tag_id: String) -> StdResult<PaymentTokenResponse> {
    let token = PAYMENT_TOKEN.load(deps.storage, &tag_id)?;
    Ok(PaymentTokenResponse { token })
}

pub fn query_next_unlock_date(deps: Deps, env: Env) -> StdResult<NextUnlockDateResponse> {
    let config = CONFIG.load(deps.storage)?;

    let current_time = env.block.time.seconds();
    let mut next_unlock_date = config.cliff;

    if current_time >= config.cliff {
        let vesting_start = if config.cliff > config.start { config.cliff } else { config.start };
        let vesting_end = vesting_start + (config.duration * Uint64::from(config.linear_vesting_count).u64());

        if current_time >= vesting_end {
            next_unlock_date = vesting_end;
        } else {
            let step_duration = config.duration;
            let mut unlock_time = vesting_start;

            for _ in 0..config.linear_vesting_count {
                unlock_time += step_duration;
                if unlock_time > current_time {
                    next_unlock_date = unlock_time;
                    break;
                }
            }
        }
    }

    Ok(NextUnlockDateResponse { next_unlock_date })
}

fn calculate_vested_amount(config: &Config, whitelist: &WhitelistInfo, current_time: u64) -> StdResult<Uint128> {
    let total_amount = whitelist.amount;
    let initial_amount = total_amount * Uint128::from(config.initial_unlock_percent) / Uint128::from(10000u16);

    if current_time < config.start {
        Ok(Uint128::zero())
    } else if config.cliff <= config.start || current_time >= config.cliff {
        calculate_linear_vesting(config, total_amount, current_time)
    } else {
        Ok(initial_amount)
    }
}

fn calculate_linear_vesting(config: &Config, total_amount: Uint128, current_time: u64) -> StdResult<Uint128> {
    let initial = total_amount * Uint128::from(config.initial_unlock_percent) / Uint128::from(10000u16);
    let remaining = total_amount - initial;

    let vesting_start = if config.cliff > config.start { config.cliff } else { config.start };
    let vesting_end = vesting_start + (config.duration * Uint64::from(config.linear_vesting_count).u64());

    if current_time >= vesting_end {
        Ok(total_amount)
    } else if current_time <= vesting_start {
        Ok(initial)
    } else {
        let time_passed = current_time - vesting_start;
        let total_vesting_time = vesting_end - vesting_start;
        let step_duration = config.duration;
        let steps_passed = time_passed / step_duration;

        if steps_passed == u64::from(config.linear_vesting_count) - 1 {
            Ok(total_amount)
        } else if current_time < vesting_start + (step_duration * (steps_passed + 1)) {
            let previous_time = vesting_start + (step_duration * steps_passed);
            let previous_vested_remaining = remaining * Uint128::from(previous_time - vesting_start) / Uint128::from(total_vesting_time);
            Ok(min(initial + previous_vested_remaining, total_amount))
        } else {
            let vested_remaining = remaining * Uint128::from(time_passed) / Uint128::from(total_vesting_time);
            let vested_amount = initial + vested_remaining;
            Ok(min(vested_amount, total_amount))
        }
    }
}

fn calculate_claimable_amount(config: &Config, whitelist: &WhitelistInfo, current_time: u64) -> StdResult<Uint128> {
    let vested_amount = calculate_vested_amount(config, whitelist, current_time)?;
    Ok(vested_amount - whitelist.distributed_amount)
}