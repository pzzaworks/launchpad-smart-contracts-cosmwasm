#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Addr, Binary, Deps, DepsMut, Empty, Env, MessageInfo, QuerierWrapper, Response, StdError, StdResult, Uint128, Uint64, WasmQuery
};
use crate::msg::{
    ExecuteMsg, InstantiateMsg, QueryMsg, GetUserTierResponse
};
use crate::state::{
    Config, CONFIG, CUSTOM_TIERS
};
use crate::ContractError;
use cw2::set_contract_version;

pub(crate) const CONTRACT_NAME: &str = "crates.io:stake_controller";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    cw_ownable::initialize_owner(deps.storage, deps.api, msg.owner.as_deref())?;

    let token_address = deps.api.addr_validate(&msg.token_address)?;
    let _: cw20::TokenInfoResponse = deps
        .querier
        .query_wasm_smart(&token_address, &cw20::Cw20QueryMsg::TokenInfo {})
        .map_err(|_| ContractError::InvalidCw20 {})?;

    let stake_contracts: Result<Vec<Addr>, _> = msg.stake_contracts.iter().map(|addr| deps.api.addr_validate(addr)).collect();
    let stake_contracts = stake_contracts?;

    if stake_contracts.len() != msg.stake_contract_multipliers.len() {
        return Err(ContractError::InvalidInputLength {});
    }

    let config = Config {
        token_address,
        stake_contracts,
        stake_contract_multipliers: msg.stake_contract_multipliers,
        tier_thresholds: msg.tier_thresholds,
    };
    
    CONFIG.save(deps.storage, &config)?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<Empty>, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig { stake_contracts, stake_contract_multipliers, tier_thresholds } => execute_update_config(deps, info, stake_contracts, stake_contract_multipliers, tier_thresholds),
        ExecuteMsg::UpdateCustomTiers { address, tier_index } => execute_update_custom_tiers(deps, info, address, tier_index),
        ExecuteMsg::UpdateOwnership(action) => execute_update_owner(deps, info, env, action),
    }
}

pub fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    stake_contracts: Vec<String>, 
    stake_contract_multipliers: Vec<Uint64>,
    tier_thresholds: Vec<Uint128>,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    if stake_contracts.len() != stake_contract_multipliers.len() {
        return Err(ContractError::InvalidInputLength {});
    }

    let stake_contracts: Result<Vec<_>, _> = stake_contracts.iter().map(|addr| deps.api.addr_validate(addr)).collect();
    let stake_contracts = stake_contracts?;

    CONFIG.update(deps.storage, |mut config| -> Result<Config, StdError> {
        config.stake_contracts = stake_contracts;
        config.stake_contract_multipliers = stake_contract_multipliers;
        config.tier_thresholds = tier_thresholds;
        Ok(config)
    })?;

    Ok(Response::new()
        .add_attribute("action", "update_config")
        .add_attribute("sender", info.sender.to_string()))
}

pub fn execute_update_custom_tiers(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
    tier_index: Uint64,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let addr = deps.api.addr_validate(&address)?;

    CUSTOM_TIERS.update(deps.storage, &addr, |_tier: Option<Uint64>| -> Result<_, StdError> {
        Ok(tier_index) 
    })?;

    Ok(Response::new()
        .add_attribute("action", "update_custom_tiers")
        .add_attribute("sender", info.sender.to_string())
        .add_attribute("address", address)
        .add_attribute("tier_index", tier_index.to_string()))
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
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_json_binary(&query_config(deps)?),
        QueryMsg::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
        QueryMsg::GetUserTierIndex { address } => to_json_binary(&query_user_tier_index(deps, env, address)?),
        QueryMsg::GetTotalStaked { address } => {
            let config = query_config(deps)?;
            let addr = deps.api.addr_validate(&address)?;
            let total_staked = query_total_staked(deps, env, &addr, &config)?;
            to_json_binary(&total_staked)
        },
        QueryMsg::GetStakedValue { address } => {
            let config = query_config(deps)?;
            let stake_amount = query_stake_amount(&deps.querier, &config.token_address, &address)?;
            to_json_binary(&stake_amount)
        },
        QueryMsg::GetUserCustomTier { address } => to_json_binary(&query_user_custom_tier(deps, env, address)?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<Config> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config)
}

pub fn query_user_tier_index(deps: Deps, env: Env, address: String) -> StdResult<GetUserTierResponse> {
    let address = deps.api.addr_validate(&address)?;
    let config = CONFIG.load(deps.storage)?;

    if let Some(custom_tier) = CUSTOM_TIERS.may_load(deps.storage, &address)?.map(|t| Uint64::from(t.u64())) {
        return Ok(GetUserTierResponse { tier: custom_tier, total_staked: Uint128::zero() });
    }

    let total_staked = query_total_staked(deps, env, &address, &config)?;

    let mut tier: Uint64 = Uint64::from(0u64);
    for (i, threshold) in config.tier_thresholds.iter().enumerate() {
        if total_staked < *threshold {
            break;
        }
        tier = Uint64::from(i as u64 + 1);
    }

    Ok(GetUserTierResponse { tier, total_staked })
}

fn query_total_staked(
    deps: Deps,
    _env: Env,
    address: &Addr,
    config: &Config,
) -> StdResult<Uint128> {
    let mut total_staked = Uint128::zero();

    for (i, contract) in config.stake_contracts.iter().enumerate() {
        let staked_amount = query_stake_amount(&deps.querier, contract, address.as_str())?;
        let multiplier = config.stake_contract_multipliers[i];
        total_staked += staked_amount * Uint128::from(multiplier) / Uint128::new(10000);
    }

    Ok(total_staked)
}

fn query_stake_amount(
    querier: &QuerierWrapper,
    stake_contract: &Addr,
    user: &str,
) -> StdResult<Uint128> {
    let msg = crate::msg::QueryMsg::GetStakedValue {
        address: user.to_string(),
    };
    let query = WasmQuery::Smart {
        contract_addr: stake_contract.to_string(),
        msg: to_json_binary(&msg)?,
    }
    .into();
    let res: crate::msg::GetStakedValueResponse = querier.query(&query)?;
    Ok(res.value)
}

pub fn query_user_custom_tier(
    deps: Deps,
    _env: Env,
    address: String,
) -> StdResult<Uint64> {
    let address = deps.api.addr_validate(&address)?;
    let user_custom_tier = CUSTOM_TIERS.may_load(deps.storage, &address)?;

    match user_custom_tier {
        Some(tier) => Ok(tier),
        None => Err(cosmwasm_std::StdError::not_found("custom_tiers")),
    }
}