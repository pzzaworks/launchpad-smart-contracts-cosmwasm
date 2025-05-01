use cosmwasm_std::{to_json_string, Order};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    entry_point, to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult, Uint128, Uint64, Coin, QuerierWrapper, WasmQuery,
};
use cw_storage_plus::Bound;
use std::convert::TryInto;
use crate::hooks::{register_hook_msgs, join_staker_round_hook_msgs, join_fcfs_round_hook_msgs};
use crate::msg::{
    ExecuteMsg, GetAllUserInfoAtHeightResponse, GetBalanceResponse, GetHooksResponse, GetStatisticsResponse, GetUserInfoAtHeightResponse, GetUserTierResponse, InstantiateMsg, QueryMsg
};
use crate::state::{
    Config, Dates, Statistics, Status, UserInfo, WhitelistProperties, BALANCE, CONFIG, HOOKS, STATISTICS, USER_INFO
};
use crate::ContractError;
use cw2::set_contract_version;
use sha2::Digest;

pub(crate) const CONTRACT_NAME: &str = "crates.io:sale";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    cw_ownable::initialize_owner(deps.storage, deps.api, msg.owner.as_deref())?;

    let payment_denom = msg.payment_denom.clone();

    let stake_controller = deps.api.addr_validate(&msg.stake_controller)?;

    let dates = Dates {
        register_start: msg.dates.register_start,
        register_end: msg.dates.register_end,
        staker_start: msg.dates.staker_start,
        staker_end: msg.dates.staker_end,
        fcfs_start: msg.dates.fcfs_start,
        fcfs_end: msg.dates.fcfs_end,
    };

    let status = Status {
        register_paused: msg.status.register_paused,
        staker_paused: msg.status.staker_paused,
        fcfs_paused: msg.status.fcfs_paused,   
    };

    let whitelist_properties = WhitelistProperties {
        whitelist_merkle_root: msg.whitelist_properties.whitelist_merkle_root,
        whitelisted_user_count: msg.whitelist_properties.whitelisted_user_count,
        whitelisted_user_allocation: msg.whitelist_properties.whitelisted_user_allocation,
    };

    let config = Config {
        owner: info.sender.clone(),
        stake_controller: stake_controller,
        payment_denom: payment_denom,
        sale_token_decimals: msg.sale_token_decimals,
        sale_token_price: msg.sale_token_price,
        min_allocation: msg.min_allocation,
        total_allocation: msg.total_allocation,
        fcfs_allocation: msg.fcfs_allocation,
        status: status,
        dates: dates,
        whitelist_properties: whitelist_properties
    };

    CONFIG.save(deps.storage, &config)?;
    BALANCE.save(deps.storage, &Uint128::zero())?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Register { proof } => execute_register(deps, env, info, proof),
        ExecuteMsg::JoinStakerRound { proof } => execute_join_staker_round(deps, env, info, proof),
        ExecuteMsg::JoinFcfsRound {} => execute_join_fcfs_round(deps, env, info),
        ExecuteMsg::UpdateConfig { stake_controller, payment_denom, sale_token_decimals, sale_token_price, min_allocation, total_allocation, fcfs_allocation, status, dates, whitelist_properties } => 
            execute_update_config(deps, info, stake_controller, payment_denom, sale_token_decimals, sale_token_price, min_allocation, total_allocation, fcfs_allocation, status, dates, whitelist_properties),        
        ExecuteMsg::Withdraw {} => execute_withdraw(deps, info),
        ExecuteMsg::AddHook { addr } => execute_add_hook(deps, env, info, addr),
        ExecuteMsg::RemoveHook { addr } => execute_remove_hook(deps, env, info, addr),
        ExecuteMsg::UpdateOwnership(action) => execute_update_owner(deps, info, env, action),
    }
}

pub fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    stake_controller_address: String,
    payment_denom: String,
    sale_token_decimals: Uint64,
    sale_token_price: Uint128,
    min_allocation: Uint128,
    total_allocation: Uint128,
    fcfs_allocation: Uint128,
    status: Status,
    dates: Dates,
    whitelist_properties: WhitelistProperties,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let stake_controller = deps.api.addr_validate(&stake_controller_address)?;

    let status_clone = status.clone();
    let dates_clone = dates.clone();
    let whitelist_properties_clone = whitelist_properties.clone();

    CONFIG.update(deps.storage, |mut config| -> Result<Config, StdError> {
        config.stake_controller = stake_controller;
        config.payment_denom = payment_denom.clone();
        config.sale_token_decimals = sale_token_decimals;
        config.sale_token_price = sale_token_price;
        config.min_allocation = min_allocation;
        config.total_allocation = total_allocation;
        config.fcfs_allocation = fcfs_allocation;
        config.status = status_clone;
        config.dates = dates_clone;
        config.whitelist_properties = whitelist_properties_clone;
        
        Ok(config)
    })?;

    Ok(Response::new()
        .add_attribute("action", "update_config")
        .add_attribute("stake_controller_address", stake_controller_address)
        .add_attribute("payment_denom", payment_denom)
        .add_attribute("sale_token_decimals", sale_token_decimals.to_string())
        .add_attribute("sale_token_price", sale_token_price.to_string())
        .add_attribute("min_allocation", min_allocation.to_string())
        .add_attribute("total_allocation", total_allocation.to_string())
        .add_attribute("status", to_json_string(&status).unwrap_or_else(|_| "Error serializing status".to_string()))
        .add_attribute("dates", to_json_string(&dates).unwrap_or_else(|_| "Error serializing dates".to_string()))
        .add_attribute("whitelist_properties", to_json_string(&whitelist_properties).unwrap_or_else(|_| "Error serializing whitelist_properties".to_string())))
}

pub fn execute_register(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    proof: Option<Vec<String>>
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if config.status.register_paused {
        return Err(ContractError::RegistrationPaused {});
    }

    let mut is_whitelisted = false;

    if config.whitelist_properties.whitelist_merkle_root.len() > 0 {
        is_whitelisted = query_verify_proof(deps.as_ref(), info.sender.to_string(), proof.unwrap_or_default()).map_err(|_| ContractError::WhitelistError {})?;
    }

    if is_whitelisted {
        return Err(ContractError::CannotRegister {});
    }

    let current_time = env.block.time.seconds();

    if current_time < config.dates.register_start.u64() {
        return Err(ContractError::RegistrationNotStarted {});
    }

    if current_time > config.dates.register_end.u64() {
        return Err(ContractError::RegistrationClosed {});
    }

    let address = info.sender.to_string();
    let user_tier_response: GetUserTierResponse = query_user_tier_index(&deps.querier, &config.stake_controller, &address)?;

    if user_tier_response.total_staked.is_zero() {
        return Err(ContractError::NotStaker {});
    }
    
    USER_INFO.update(deps.storage, &info.sender, env.block.height, |user_info: Option<UserInfo>| -> StdResult<UserInfo> {
        let mut updated_info = user_info.unwrap_or_default();
        if updated_info.registered {
            return Err(StdError::generic_err("Already registered"));
        }
        updated_info.address = info.sender.clone();
        updated_info.registered = true;
        updated_info.tier = user_tier_response.tier;
        updated_info.total_staked = user_tier_response.total_staked;
        Ok(updated_info)
    })?;

    STATISTICS.update(deps.storage, env.block.height, |stats: Option<Statistics>| -> StdResult<Statistics> {
        let mut updated_stats = stats.unwrap_or_default();
        updated_stats.total_registered_users += Uint128::new(1);
        updated_stats.total_staked += user_tier_response.total_staked;
        Ok(updated_stats)
    })?;

    let hook_msgs = register_hook_msgs(deps.storage, info.sender.clone(), user_tier_response.tier, user_tier_response.total_staked)?;

    Ok(Response::new()
        .add_submessages(hook_msgs)
        .add_attribute("method", "register")
        .add_attribute("user", info.sender)
        .add_attribute("tier", user_tier_response.tier.to_string())
        .add_attribute("total_staked", user_tier_response.total_staked.to_string()))
}

pub fn execute_join_staker_round(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    proof: Option<Vec<String>>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let balance = BALANCE.load(deps.storage).unwrap_or_default();

    let payment_amount = info.funds[0].amount;

    if info.funds.len() != 1 || info.funds[0].denom != config.payment_denom {
        return Err(ContractError::InvalidDenom {});
    }

    if config.status.staker_paused {
        return Err(ContractError::StakerRoundPaused {});
    }

    if payment_amount.is_zero() {
        return Err(ContractError::InvalidAmount {});
    }

    if payment_amount < config.min_allocation {
        return Err(ContractError::MinAllocationNotReached {});
    }

    let current_time = env.block.time.seconds();

    if current_time < config.dates.staker_start.u64() {
        return Err(ContractError::StakerRoundNotStarted {});
    }
    
    if current_time > config.dates.staker_end.u64() {
        return Err(ContractError::StakerRoundClosed {});
    }

    let mut is_whitelisted = false;

    if config.whitelist_properties.whitelist_merkle_root.len() > 0 {
        is_whitelisted = query_verify_proof(deps.as_ref(), info.sender.to_string(), proof.clone().unwrap_or_default()).map_err(|_| ContractError::WhitelistError {})?;
    }

    let user_allocation = query_user_staker_allocation(deps.as_ref(), env.clone(), info.sender.to_string(), proof)?;

    if user_allocation.is_zero() {
        return Err(ContractError::NoAllocation {});
    }

    if balance.checked_add(payment_amount).unwrap() > config.total_allocation {
        return Err(ContractError::ExceedTotalAllocation {});
    }

    let sale_token_amount = (payment_amount.u128() * 10u128.pow(config.sale_token_decimals.u64() as u32)) / config.sale_token_price.u128();
    let sale_token_amount = Uint128::from(sale_token_amount);
    let prev_user_info = USER_INFO.load(deps.storage, &info.sender).unwrap_or_default();

    STATISTICS.update(deps.storage, env.block.height, |stats: Option<Statistics>| -> StdResult<Statistics> {
        let mut updated_stats = stats.unwrap_or_default();

        if !prev_user_info.joined_staker_round && !prev_user_info.joined_fcfs_round {
            updated_stats.total_participants += Uint128::new(1);
        }
    
        if !prev_user_info.joined_staker_round {
            updated_stats.total_staker_round_participants += Uint128::new(1);
        }

        updated_stats.total_payment_amount += payment_amount;
        
        Ok(updated_stats)
    })?;
    
    USER_INFO.update(deps.storage, &info.sender, env.block.height, |user_info: Option<UserInfo>| -> StdResult<UserInfo> {
        let mut updated_info = user_info.unwrap_or_else(|| UserInfo {
            address: info.sender.clone(),
            ..Default::default()
        });
        
        if !is_whitelisted {
            if updated_info.total_staked.is_zero() {
                return Err(StdError::generic_err("Not registered"));
            }
        
            if !updated_info.registered {
                return Err(StdError::generic_err("Not registered"));
            }
        }

        updated_info.total_payment_amount += payment_amount;
        updated_info.total_sale_token_amount += sale_token_amount;

        if updated_info.total_payment_amount > user_allocation {
            return Err(StdError::generic_err("Exceed user allocation"));
        } 

        if updated_info.total_payment_amount > config.total_allocation {
            return Err(StdError::generic_err("Exceed total allocation"));
        }

        updated_info.joined_staker_round = true;

        Ok(updated_info)
    })?;

    BALANCE.update(deps.storage, |balance: Uint128| -> StdResult<Uint128> {
        Ok(balance + payment_amount)
    })?;

    let hook_msgs = join_staker_round_hook_msgs(deps.storage, info.sender.clone(), payment_amount)?;

    Ok(Response::new()
        .add_submessages(hook_msgs)
        .add_attribute("method", "join_staker_round")
        .add_attribute("user", info.sender.to_string())
        .add_attribute("payment_amount", payment_amount.to_string())
        .add_attribute("sale_token_amount", sale_token_amount.to_string()))
}

pub fn execute_join_fcfs_round(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let balance = BALANCE.load(deps.storage).unwrap_or_default();

    let payment_amount = info.funds[0].amount;

    if info.funds.len() != 1 || info.funds[0].denom != config.payment_denom {
        return Err(ContractError::InvalidDenom {});
    }

    if config.status.fcfs_paused {
        return Err(ContractError::FcfsRoundPaused {});
    }

    if payment_amount.is_zero() {
        return Err(ContractError::InvalidAmount {});
    }

    if payment_amount < config.min_allocation {
        return Err(ContractError::MinAllocationNotReached {});
    }

    let current_time = env.block.time.seconds();

    if current_time < config.dates.fcfs_start.u64() {
        return Err(ContractError::FcfsRoundNotStarted {});
    }
    
    if current_time > config.dates.fcfs_end.u64() {
        return Err(ContractError::FcfsRoundClosed {});
    }
    
    let user_allocation = query_user_fcfs_allocation(deps.as_ref(), env.clone())?;

    if user_allocation.is_zero() {
        return Err(ContractError::NoAllocation {});
    }

    if balance.checked_add(payment_amount).unwrap() > config.total_allocation {
        return Err(ContractError::ExceedTotalAllocation {});
    }

    let sale_token_amount = (payment_amount.u128() * 10u128.pow(config.sale_token_decimals.u64() as u32)) / config.sale_token_price.u128();
    let sale_token_amount = Uint128::from(sale_token_amount);
    let prev_user_info: UserInfo = USER_INFO.load(deps.storage, &info.sender).unwrap_or_default();

    STATISTICS.update(deps.storage, env.block.height, |stats: Option<Statistics>| -> StdResult<Statistics> {
        let mut updated_stats = stats.unwrap_or_default();

        if !prev_user_info.joined_staker_round && !prev_user_info.joined_fcfs_round {
            updated_stats.total_participants += Uint128::new(1);
        }

        if !prev_user_info.joined_fcfs_round {
            updated_stats.total_fcfs_round_participants += Uint128::new(1);
        }

        updated_stats.total_payment_amount += payment_amount;
        
        Ok(updated_stats)
    })?;

    USER_INFO.update(deps.storage, &info.sender, env.block.height, |user_info: Option<UserInfo>| -> StdResult<UserInfo> {
        let mut updated_info = user_info.unwrap_or_else(|| UserInfo {
            address: info.sender.clone(),
            ..Default::default()
        });
        
        updated_info.total_payment_amount += payment_amount;
        updated_info.total_sale_token_amount += sale_token_amount;

        if updated_info.total_payment_amount > user_allocation {
            return Err(StdError::generic_err("Exceed user allocation"));
        }

        if updated_info.total_payment_amount > config.total_allocation {
            return Err(StdError::generic_err("Exceed total allocation"));
        }

        updated_info.joined_fcfs_round = true;

        Ok(updated_info)
    })?;

    BALANCE.update(deps.storage, |balance: Uint128| -> StdResult<Uint128> {
        Ok(balance + payment_amount)
    })?;

    let hook_msgs = join_fcfs_round_hook_msgs(deps.storage, info.sender.clone(), payment_amount)?;

    Ok(Response::new()
        .add_submessages(hook_msgs)
        .add_attribute("method", "join_fcfs_round")
        .add_attribute("user", info.sender.to_string())
        .add_attribute("payment_amount", payment_amount.to_string())
        .add_attribute("sale_token_amount", sale_token_amount.to_string()))
}

pub fn execute_withdraw(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
    let config = CONFIG.load(deps.storage)?;
    let balance = BALANCE.load(deps.storage)?;

    let msg = cosmwasm_std::BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![Coin {
            denom: config.payment_denom,
            amount: balance,
        }],
    };

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "withdraw")
        .add_attribute("from", info.sender.to_string())
        .add_attribute("amount", balance.to_string()))
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    let res = match  msg {
        QueryMsg::GetConfig {} => to_json_binary(&query_config(deps)?),
        QueryMsg::GetUserTierIndex { address } => {
            let config = query_config(deps)?;
            let user_tier_index = query_user_tier_index(&deps.querier, &config.stake_controller, &address)?;
            to_json_binary(&user_tier_index)
        },
        QueryMsg::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
        QueryMsg::GetHooks {} => to_json_binary(&query_hooks(deps)?),
        QueryMsg::GetBalance {} => to_json_binary(&query_balance(deps, env)?),
        QueryMsg::GetStatistics { height } => to_json_binary(&query_statistics_at_height(deps, env, height)?),
        QueryMsg::GetUserInfoAtHeight { address, height } => to_json_binary(&query_user_info_at_height(deps, env, address, height)?),
        QueryMsg::GetAllUserInfoAtHeight { start_after, limit, height } => to_json_binary(&query_all_user_info_at_height(deps, env, start_after, limit, height)?),
        QueryMsg::GetUserStakerAllocation { address, proof } => to_json_binary(&query_user_staker_allocation(deps, env, address, proof)?),
        QueryMsg::GetUserFCFSAllocation {} => to_json_binary(&query_user_fcfs_allocation(deps, env)?),
        QueryMsg::VerifyProof { address, proof } => to_json_binary(&query_verify_proof(deps, address, proof)?),
    };

    res.map_err(|e| ContractError::QueryError { msg: e.to_string() })
}


pub fn query_config(deps: Deps) -> StdResult<Config> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config)
}

pub fn query_hooks(deps: Deps) -> StdResult<GetHooksResponse> {
    Ok(GetHooksResponse {
        hooks: HOOKS.query_hooks(deps)?.hooks,
    })
}

pub fn query_balance(
    deps: Deps,
    _env: Env,
) -> StdResult<GetBalanceResponse> {
    let balance = BALANCE.load(deps.storage)?;
    Ok(GetBalanceResponse { balance })
}

pub fn query_statistics_at_height(
    deps: Deps,
    env: Env,
    height: Option<u64>,
) -> StdResult<GetStatisticsResponse> {
    let height = height.unwrap_or(env.block.height);
    let statistics = STATISTICS.may_load_at_height(deps.storage, env.block.height)?.unwrap_or_default();
    Ok(GetStatisticsResponse { statistics, height: Uint64::from(height) })
}

pub fn query_user_info_at_height(
    deps: Deps,
    env: Env,
    address: String,
    height: Option<u64>,
) -> StdResult<GetUserInfoAtHeightResponse> {
    let address = deps.api.addr_validate(&address)?;
    let height = height.unwrap_or(env.block.height);
    let user_info = USER_INFO
        .may_load_at_height(deps.storage, &address, height)?
        .unwrap_or_default();
    Ok(GetUserInfoAtHeightResponse { user_info, height: Uint64::from(height) })
}
    
pub fn query_all_user_info_at_height(
    deps: Deps,
    env: Env,
    start_after: Option<String>,
    limit: Option<u32>,
    height: Option<u64>,
) -> StdResult<GetAllUserInfoAtHeightResponse> {
    let height = height.unwrap_or(env.block.height);
    let limit = limit.unwrap_or(50).min(1000) as usize;
    
    let start: Option<Addr> = start_after
        .clone()  
        .map(|s| deps.api.addr_validate(&s))
        .transpose()?;

    let user_infos: Vec<UserInfo> = USER_INFO
        .range(
            deps.storage, 
            start.as_ref().map(Bound::exclusive),
            None,
            Order::Ascending
        )
        .take(limit)
        .filter_map(|item| {
            item.ok().and_then(|(addr, _)| {
                USER_INFO.may_load_at_height(deps.storage, &addr, height).unwrap_or(None)
            })
        })
        .collect();

    Ok(GetAllUserInfoAtHeightResponse {
        user_infos,
        height: Uint64::from(height),
    })
}

pub fn query_user_tier_index(querier: &QuerierWrapper, stake_controller_contract: &Addr, address: &str) -> StdResult<GetUserTierResponse> {
    let msg = crate::msg::QueryMsg::GetUserTierIndex {
        address: address.to_string(),
    };
    let query = WasmQuery::Smart {
        contract_addr: stake_controller_contract.to_string(),
        msg: to_json_binary(&msg)?,
    }
    .into();
    let res: crate::msg::GetUserTierResponse = querier.query(&query)?;
    Ok(res)
}

pub fn query_user_staker_allocation(deps: Deps, _env: Env, address: String, proof: Option<Vec<String>>) -> Result<Uint128, ContractError> {
    let prev_address = deps.api.addr_validate(&address)?;
    let config = CONFIG.load(deps.storage)?;
    let statistics = STATISTICS.may_load(deps.storage)?.unwrap_or_default();
    let user_total_staked = USER_INFO.may_load(deps.storage, &prev_address)?.unwrap_or_default().total_staked;

    if user_total_staked.is_zero() {
        return Ok(Uint128::zero());
    }

    let mut user_allocation = (config.total_allocation * user_total_staked) / statistics.total_staked;
    
    let mut is_whitelisted = false;

    if config.whitelist_properties.whitelist_merkle_root.len() > 0 {
        is_whitelisted = query_verify_proof(deps, address, proof.unwrap_or_default()).map_err(|_| ContractError::WhitelistError {})?;

        user_allocation = ((config.total_allocation - (config.whitelist_properties.whitelisted_user_allocation * config.whitelist_properties.whitelisted_user_count)) * statistics.total_staked) / user_total_staked;
    }

    if is_whitelisted {
        user_allocation = config.whitelist_properties.whitelisted_user_allocation;
    }

    Ok(user_allocation)
}

pub fn query_user_fcfs_allocation(deps: Deps, _env: Env) -> Result<Uint128, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let statistics = STATISTICS.may_load(deps.storage)?.unwrap_or_default();

    let mut user_allocation = config.total_allocation - statistics.total_payment_amount;

    if config.fcfs_allocation > Uint128::new(0) {
        user_allocation = config.fcfs_allocation;
    }

    Ok(user_allocation)
}

pub fn query_verify_proof(deps: Deps, sender: String, proof: Vec<String>) -> Result<bool, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;
    let user_input = format!("{}{}", sender, config.whitelist_properties.whitelisted_user_allocation);
    let hash: [u8; 32] = sha2::Sha256::digest(user_input.as_bytes()).into();
    
    let hash = proof.into_iter().try_fold(hash, |hash, p| {
        let mut proof_buf = [0; 32];
        hex::decode_to_slice(p, &mut proof_buf)?;
        let mut hashes = [hash, proof_buf];
        hashes.sort_unstable();
        sha2::Sha256::digest(&hashes.concat())
            .as_slice()
            .try_into()
            .map_err(|_| ContractError::WrongLength {})
    })?;

    let mut root_buf: [u8; 32] = [0; 32];
    hex::decode_to_slice(config.whitelist_properties.whitelist_merkle_root, &mut root_buf)?;
    Ok(root_buf == hash)
}