#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_json, to_json_binary, Addr, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdError, StdResult, Timestamp, Uint128, Uint64
};
use cw20::{Cw20ReceiveMsg, TokenInfoResponse};
use crate::hooks::{emergency_unstake_hook_msgs, harvest_hook_msgs, reinvest_hook_msgs, stake_hook_msgs, unstake_hook_msgs};
use crate::msg::{
    ExecuteMsg, GetHooksResponse, InstantiateMsg, ListStakersResponse, GetNextUnlockTimeResponse, QueryMsg, ReceiveMsg, GetRewardBalanceResponse, GetRewardAmountResponse, GetStakedBalanceAtHeightResponse, GetStakedTimeResponse, GetStakedValueResponse, GetStakerBalanceResponse, GetTotalStakedAtHeightResponse, GetTotalStakersAtHeightResponse, GetTotalValueResponse
};
use crate::state::{
    Config, REWARD_BALANCE, CONFIG, HOOKS, STAKED_BALANCES, STAKED_TIMES, STAKED_TOTAL, STAKERS_TOTAL
};
use crate::ContractError;
use cw2::set_contract_version;

pub(crate) const CONTRACT_NAME: &str = "crates.io:stake";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<Empty>, ContractError> {
    cw_ownable::initialize_owner(deps.storage, deps.api, msg.owner.as_deref())?;
    let token_address = deps.api.addr_validate(&msg.token_address)?;
    let _: TokenInfoResponse = deps
        .querier
        .query_wasm_smart(&token_address, &cw20::Cw20QueryMsg::TokenInfo {})
        .map_err(|_| ContractError::InvalidCw20 {})?;
    let fee_address = deps.api.addr_validate(&msg.fee_address)?;

    let config = Config {
        token_address,
        stake_paused: msg.stake_paused, 
        unstake_paused: msg.unstake_paused,
        emergency_unstake_paused: msg.emergency_unstake_paused,
        interest_rate: msg.interest_rate,
        lock_duration: msg.lock_duration, 
        lock_duration_multiplier: msg.lock_duration_multiplier,
        emergency_unstake_fee_percentage: msg.emergency_unstake_fee_percentage,
        fee_address
    };

    CONFIG.save(deps.storage, &config)?;
    STAKED_TOTAL.save(deps.storage, &Uint128::zero(), env.block.height)?;
    STAKERS_TOTAL.save(deps.storage, &Uint128::zero(), env.block.height)?;
    REWARD_BALANCE.save(deps.storage, &Uint128::zero())?;

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
        ExecuteMsg::Receive(msg) => execute_receive(deps, env, info, msg),
        ExecuteMsg::Unstake { amount } => execute_unstake(deps, env, info, amount),
        ExecuteMsg::EmergencyUnstake { amount } => execute_emergency_unstake(deps, env, info, amount),
        ExecuteMsg::Harvest {} => execute_harvest(deps, env, info),
        ExecuteMsg::ReInvest {} => execute_reinvest(deps, env, info),
        ExecuteMsg::UpdateConfig { stake_paused, unstake_paused, emergency_unstake_paused, interest_rate, lock_duration, lock_duration_multiplier, emergency_unstake_fee_percentage, fee_address } => execute_update_config(info, deps, stake_paused, unstake_paused, emergency_unstake_paused, interest_rate, lock_duration, lock_duration_multiplier, emergency_unstake_fee_percentage, fee_address),
        ExecuteMsg::Withdraw {} => execute_withdraw(deps, info),
        ExecuteMsg::AddHook { addr } => execute_add_hook(deps, env, info, addr),
        ExecuteMsg::RemoveHook { addr } => execute_remove_hook(deps, env, info, addr),
        ExecuteMsg::UpdateOwnership(action) => execute_update_owner(deps, info, env, action),
    }
}

pub fn execute_update_config(
    info: MessageInfo,
    deps: DepsMut,
    stake_paused: bool, 
    unstake_paused: bool,
    emergency_unstake_paused: bool,
    interest_rate: Uint64, 
    lock_duration: Uint64,
    lock_duration_multiplier: Uint64,
    emergency_unstake_fee_percentage: Uint64,
    fee_address: String
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    if interest_rate == Uint64::zero() {
        return Err(ContractError::InvalidInterestRate {});
    }

    let fee_address_addr = deps.api.addr_validate(&fee_address)?; 

    CONFIG.update(deps.storage, |mut config| -> Result<Config, StdError> {
        config.stake_paused = stake_paused;
        config.unstake_paused = unstake_paused;
        config.emergency_unstake_paused = emergency_unstake_paused;
        config.interest_rate = interest_rate;
        config.lock_duration = lock_duration;
        config.lock_duration_multiplier = lock_duration_multiplier;
        config.emergency_unstake_fee_percentage = emergency_unstake_fee_percentage;
        config.fee_address = fee_address_addr;
        Ok(config)
    })?;

    Ok(Response::new()
        .add_attribute("action", "update_config")
        .add_attribute("stake_paused", stake_paused.to_string())
        .add_attribute("unstake_paused", unstake_paused.to_string())
        .add_attribute("emergency_unstake_paused", emergency_unstake_paused.to_string())
        .add_attribute("interest_rate", interest_rate.to_string())
        .add_attribute("lock_duration", lock_duration.to_string())
        .add_attribute("lock_duration_multiplier", lock_duration_multiplier.to_string())
        .add_attribute("emergency_unstake_fee_percentage", emergency_unstake_fee_percentage.to_string()))
}

pub fn execute_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    wrapper: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.token_address {
        return Err(ContractError::InvalidToken {
            received: info.sender,
            expected: config.token_address,
        });
    }
    let msg: ReceiveMsg = from_json(&wrapper.msg)?;
    let sender = deps.api.addr_validate(&wrapper.sender)?;
    match msg {
        ReceiveMsg::Stake {} => execute_stake(deps, env, sender, wrapper.amount),
        ReceiveMsg::AddTokens {} => execute_add_tokens(deps, env, sender, wrapper.amount),
    }
}

pub fn execute_add_tokens(
    deps: DepsMut,
    _env: Env,
    sender: Addr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    if amount == Uint128::new(0) {
        return Err(ContractError::InvalidAmount {});
    }

    REWARD_BALANCE.update(deps.storage, |reward_balance: Uint128| -> StdResult<_> {
        reward_balance.checked_add(amount).map_err(StdError::overflow)
    })?;

    Ok(Response::new()
        .add_attribute("action", "add_tokens")
        .add_attribute("from", sender)
        .add_attribute("amount", amount))
}

pub fn execute_stake(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let reward_balance = REWARD_BALANCE.load(deps.storage)?;

    if config.stake_paused {
        return Err(ContractError::StakePaused {});
    }

    if amount == Uint128::new(0) {
        return Err(ContractError::InvalidAmount {});
    }

    let mut amount_to_stake: Uint128 = amount;
    let user_staked_total = STAKED_BALANCES
        .load(deps.storage, &sender)
        .unwrap_or_default();

    if user_staked_total.u128() > 0 {
        let reward_amount_response: GetRewardAmountResponse = query_reward_amount(deps.as_ref(), env.clone(), sender.to_string())?;
        let reward_amount = reward_amount_response.reward_amount.u128();
        
        if reward_amount > reward_balance.u128() {
            return Err(ContractError::InsufficientRewardBalance {});
        }

        REWARD_BALANCE.update(deps.storage, |reward_balance: Uint128| -> StdResult<Uint128> {
            Ok(reward_balance.checked_sub(Uint128::from(reward_amount))?)
        })?;
        
        amount_to_stake = Uint128::from(amount.u128() + reward_amount);
    } else {
        STAKERS_TOTAL.update(
            deps.storage,
            env.block.height,
            |total| -> StdResult<Uint128> {
                Ok(total.unwrap_or_default().checked_add(Uint128::from(1u128))?)
            },
        )?;
    }

    STAKED_TIMES.update(
        deps.storage,
        &sender,
        env.block.height,
        |_time| -> StdResult<Timestamp> { Ok(env.block.time) }
    )?;

    STAKED_BALANCES.update(
        deps.storage,
        &sender,
        env.block.height,
        |bal| -> StdResult<Uint128> { Ok(bal.unwrap_or_default().checked_add(amount_to_stake)?) },
    )?;

    STAKED_TOTAL.update(
        deps.storage,
        env.block.height,
        |total| -> StdResult<Uint128> {
            Ok(total.unwrap_or_default().checked_add(amount_to_stake)?)
        },
    )?;

    let hook_msgs = stake_hook_msgs(deps.storage, sender.clone(), amount_to_stake)?;
    Ok(Response::new()
        .add_submessages(hook_msgs)
        .add_attribute("action", "stake")
        .add_attribute("from", sender)
        .add_attribute("amount", amount_to_stake.to_string()))
}

pub fn execute_unstake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let reward_balance = REWARD_BALANCE.load(deps.storage)?;
    let staked_total = STAKED_TOTAL.load(deps.storage)?;
    let prev_staked_time: Timestamp = STAKED_TIMES
        .load(deps.storage, &info.sender)
        .unwrap_or_default();
    let user_staked_total = STAKED_BALANCES
        .load(deps.storage, &info.sender)
        .unwrap_or_default();

    if config.unstake_paused {
        return Err(ContractError::UnstakePaused {});
    }

    if amount == Uint128::new(0) {
        return Err(ContractError::InvalidAmount {});
    }

    if staked_total.is_zero() {
        return Err(ContractError::NothingStaked {});
    }
    
    if amount > staked_total {
        return Err(ContractError::ImpossibleUnstake {});
    }

    if amount > user_staked_total {
        return Err(ContractError::ImpossibleUnstake {});
    }

    if user_staked_total == Uint128::new(0) {
        return Err(ContractError::NothingStaked {});
    }

    let block_time = env.block.time.seconds();
    let next_unlock_time = prev_staked_time.seconds() + config.lock_duration.u64();

    if block_time < next_unlock_time {
        return Err(ContractError::LockDurationNotPassed {});
    }

    if amount == user_staked_total {
        STAKERS_TOTAL.update(
            deps.storage,
            env.block.height,
            |total| -> StdResult<Uint128> {
                Ok(total.unwrap_or_default().checked_sub(Uint128::from(1u128))?)
            },
        )?;
    }

    STAKED_TIMES.update(
        deps.storage,
        &info.sender,
        env.block.height,
        |_time| -> StdResult<Timestamp> { Ok(env.block.time) }
    )?;

    STAKED_BALANCES.update(
        deps.storage,
        &info.sender,
        env.block.height,
        |bal| -> StdResult<Uint128> { Ok(bal.unwrap_or_default().checked_sub(amount)?) },
    )?;
    
    STAKED_TOTAL.update(
        deps.storage,
        env.block.height,
        |total| -> StdResult<Uint128> {
            Ok(total.unwrap_or_default().checked_sub(amount)?)
        },
    )?;
    
    let hook_msgs = unstake_hook_msgs(deps.storage, info.sender.clone(), amount)?;

    let reward_amount_response: GetRewardAmountResponse = query_reward_amount(deps.as_ref(), env.clone(), info.sender.to_string())?;
    let reward_amount = reward_amount_response.reward_amount.u128();

    if reward_amount > reward_balance.u128() {
        return Err(ContractError::InsufficientRewardBalance {});
    }

    REWARD_BALANCE.update(deps.storage, |reward_balance: Uint128| -> StdResult<Uint128> {
        Ok(reward_balance.checked_sub(Uint128::from(reward_amount))?)
    })?;

    let total_amount = Uint128::from(amount.u128() + reward_amount);

    let cw_send_msg = cw20::Cw20ExecuteMsg::Transfer {
        recipient: info.sender.to_string(),
        amount: total_amount,
    };

    let wasm_msg = cosmwasm_std::WasmMsg::Execute {
        contract_addr: config.token_address.to_string(),
        msg: to_json_binary(&cw_send_msg)?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_message(wasm_msg)
        .add_submessages(hook_msgs)
        .add_attribute("action", "unstake")
        .add_attribute("from", info.sender)
        .add_attribute("amount", amount)
        .add_attribute("reward_amount", reward_amount.to_string())
        .add_attribute("total_amount", total_amount.to_string()))
}

pub fn execute_emergency_unstake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let staked_total = STAKED_TOTAL.load(deps.storage)?;
    let user_staked_total = STAKED_BALANCES
        .load(deps.storage, &info.sender)
        .unwrap_or_default();

    if config.emergency_unstake_paused {
        return Err(ContractError::EmergencyUnstakePaused {});
    }

    if amount == Uint128::new(0) {
        return Err(ContractError::InvalidAmount {});
    }

    if staked_total.is_zero() {
        return Err(ContractError::NothingStaked {});
    }

    if amount > staked_total {
        return Err(ContractError::ImpossibleUnstake {});
    }

    if amount > user_staked_total {
        return Err(ContractError::ImpossibleUnstake {});
    }

    if user_staked_total == Uint128::new(0) {
        return Err(ContractError::NothingStaked {});
    }

    if amount == user_staked_total {
        STAKERS_TOTAL.update(
            deps.storage,
            env.block.height,
            |total| -> StdResult<Uint128> {
                Ok(total.unwrap_or_default().checked_sub(Uint128::from(1u128))?)
            },
        )?;
    }

    let fee_percentage = config.emergency_unstake_fee_percentage;
    let fee_amount: Uint128 = amount * Uint128::from(fee_percentage) / Uint128::from(10000u64);
    let amount_after_fee = amount.checked_sub(fee_amount).map_err(|_| ContractError::InvalidAmount {  })?;

    STAKED_BALANCES.update(
        deps.storage,
        &info.sender,
        env.block.height,
        |bal| -> StdResult<Uint128> { Ok(bal.unwrap_or_default().checked_sub(amount)?) },
    )?;

    STAKED_TOTAL.update(
        deps.storage,
        env.block.height,
        |total| -> StdResult<Uint128> {
            Ok(total.unwrap_or_default().checked_sub(amount)?)
        },
    )?;

    let cw_send_msg = cw20::Cw20ExecuteMsg::Transfer {
        recipient: info.sender.to_string(),
        amount: amount_after_fee,
    };

    let wasm_msg = cosmwasm_std::WasmMsg::Execute {
        contract_addr: config.token_address.to_string(),
        msg: to_json_binary(&cw_send_msg)?,
        funds: vec![],
    };

    let cw_send_msg_fee = cw20::Cw20ExecuteMsg::Transfer {
        recipient: config.fee_address.to_string(),
        amount: fee_amount,
    };

    let wasm_msg_fee = cosmwasm_std::WasmMsg::Execute {
        contract_addr: config.token_address.to_string(),
        msg: to_json_binary(&cw_send_msg_fee)?,
        funds: vec![],
    };

    let hook_msgs = emergency_unstake_hook_msgs(deps.storage, info.sender.clone(), amount_after_fee)?;

    Ok(Response::new()
        .add_message(wasm_msg)
        .add_message(wasm_msg_fee)
        .add_submessages(hook_msgs)
        .add_attribute("action", "emergency_unstake")
        .add_attribute("from", info.sender)
        .add_attribute("amount", amount)
        .add_attribute("fee_amount", fee_amount.to_string())
        .add_attribute("amount_after_fee", amount_after_fee.to_string()))
}

pub fn execute_harvest(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let user_staked_total = STAKED_BALANCES
        .load(deps.storage, &info.sender)
        .unwrap_or_default();

    if user_staked_total.is_zero() {
        return Err(ContractError::NothingStaked {});
    }

    let reward_amount_response: GetRewardAmountResponse = query_reward_amount(deps.as_ref(), env.clone(), info.sender.to_string())?;
    let reward_amount: u128 = reward_amount_response.reward_amount.u128();
    let reward_amount_uint128 = Uint128::from(reward_amount);

    if reward_amount_uint128.is_zero() {
        return Err(ContractError::NoRewardsToHarvest {});
    }

    STAKED_TIMES.update(
        deps.storage,
        &info.sender,
        env.block.height,
        |_time| -> StdResult<Timestamp> { Ok(env.block.time) }
    )?;

    let reward_balance = REWARD_BALANCE.load(deps.storage)?;

    if reward_balance < reward_amount_uint128 {
        return Err(ContractError::InsufficientRewardBalance {});
    }

    REWARD_BALANCE.update(deps.storage, |mut reward_balance: Uint128| -> StdResult<Uint128> {
        reward_balance = reward_balance.checked_sub(reward_amount_uint128)?;
        Ok(reward_balance)
    })?;

    let cw_send_msg = cw20::Cw20ExecuteMsg::Transfer {
        recipient: info.sender.to_string(),
        amount: reward_amount_uint128,
    };

    let wasm_msg = cosmwasm_std::WasmMsg::Execute {
        contract_addr: config.token_address.to_string(),
        msg: to_json_binary(&cw_send_msg)?,
        funds: vec![],
    };

    let hook_msgs = harvest_hook_msgs(deps.storage, info.sender.clone(), reward_amount_uint128)?;
    
    Ok(Response::new()
        .add_message(wasm_msg)
        .add_submessages(hook_msgs)
        .add_attribute("action", "harvest")
        .add_attribute("from", info.sender.to_string())
        .add_attribute("reward_amount", reward_amount_uint128.to_string()))
}

pub fn execute_reinvest(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let user_staked_total = STAKED_BALANCES
        .load(deps.storage, &info.sender)
        .unwrap_or_default();

    if user_staked_total.is_zero() {
        return Err(ContractError::NothingStaked {});
    }

    let reward_amount_response: GetRewardAmountResponse = query_reward_amount(deps.as_ref(), env.clone(), info.sender.to_string())?;
    let reward_amount: u128 = reward_amount_response.reward_amount.u128();
    let reward_amount_uint128 = Uint128::from(reward_amount);

    if reward_amount_uint128.is_zero() {
        return Err(ContractError::NoRewardsToReInvest {});
    }

    STAKED_TIMES.update(
        deps.storage,
        &info.sender,
        env.block.height,
        |_time| -> StdResult<Timestamp> { Ok(env.block.time) }
    )?;

    STAKED_BALANCES.update(
        deps.storage,
        &info.sender,
        env.block.height,
        |bal| -> StdResult<Uint128> { Ok(bal.unwrap_or_default().checked_add(reward_amount_uint128)?) },
    )?;

    STAKED_TOTAL.update(
        deps.storage,
        env.block.height,
        |total| -> StdResult<Uint128> { Ok(total.unwrap_or_default().checked_add(reward_amount_uint128)?) },
    )?;

    REWARD_BALANCE.update(deps.storage, |mut reward_balance: Uint128| -> StdResult<Uint128> {
        reward_balance = reward_balance.checked_sub(reward_amount_uint128).map_err(StdError::overflow)?;
        Ok(reward_balance)
    })?;

    let hook_msgs = reinvest_hook_msgs(deps.storage, info.sender.clone(), reward_amount_uint128)?;

    Ok(Response::new()
        .add_submessages(hook_msgs)
        .add_attribute("action", "reinvest")
        .add_attribute("from", info.sender.to_string())
        .add_attribute("reward_amount", reward_amount_uint128.to_string()))
}

pub fn execute_withdraw(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
    let config = CONFIG.load(deps.storage)?;
    let staked_total = STAKED_TOTAL.load(deps.storage)?;
    let reward_balance = REWARD_BALANCE.load(deps.storage)?;

    let total_balance = staked_total + reward_balance;

    let cw_send_msg = cw20::Cw20ExecuteMsg::Transfer {
        recipient: info.sender.to_string(),
        amount: total_balance,
    };

    let wasm_msg = cosmwasm_std::WasmMsg::Execute {
        contract_addr: config.token_address.to_string(),
        msg: to_json_binary(&cw_send_msg)?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_message(wasm_msg)
        .add_attribute("action", "withdraw")
        .add_attribute("from", info.sender.to_string())
        .add_attribute("amount", total_balance.to_string()))
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
    let res = match msg {
        QueryMsg::GetConfig {} => to_json_binary(&query_config(deps)?),
        QueryMsg::GetStakedBalanceAtHeight { address, height } => {
            to_json_binary(&query_staked_balance_at_height(deps, env, address, height)?)
        }
        QueryMsg::GetTotalStakedAtHeight { height } => {
            to_json_binary(&query_total_staked_at_height(deps, env, height)?)
        }
        QueryMsg::GetTotalStakersAtHeight { height } => {
            to_json_binary(&query_total_stakers_at_height(deps, env, height)?)
        }
        QueryMsg::GetStakedValue { address } => to_json_binary(&query_staked_value(deps, env, address)?),
        QueryMsg::GetStakedTime { address } => to_json_binary(&query_staked_time(deps, env, address)?),
        QueryMsg::GetRewardAmount { address } => to_json_binary(&query_reward_amount(deps, env, address)?),
        QueryMsg::GetNextUnlockTime { address } => to_json_binary(&query_next_unlock_time(deps, env, address)?),
        QueryMsg::GetTotalValue {} => to_json_binary(&query_total_value(deps, env)?),
        QueryMsg::GetHooks {} => to_json_binary(&query_hooks(deps)?),
        QueryMsg::ListStakers { start_after, limit } => {
            query_list_stakers(deps, start_after, limit)
        }
        QueryMsg::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
        QueryMsg::GetRewardBalance {} => to_json_binary(&query_reward_balance(deps, env)?),
    };

    res.map_err(|e| ContractError::QueryError { msg: e.to_string() })
}

pub fn query_reward_balance(
    deps: Deps,
    _env: Env,
) -> StdResult<GetRewardBalanceResponse> {
    let reward_balance = REWARD_BALANCE.load(deps.storage)?;
    Ok(GetRewardBalanceResponse { reward_balance })
}

pub fn query_staked_balance_at_height(
    deps: Deps,
    env: Env,
    address: String,
    height: Option<u64>,
) -> StdResult<GetStakedBalanceAtHeightResponse> {
    let address = deps.api.addr_validate(&address)?;
    let height = height.unwrap_or(env.block.height);
    let balance = STAKED_BALANCES
        .may_load_at_height(deps.storage, &address, height)?
        .unwrap_or_default();
    Ok(GetStakedBalanceAtHeightResponse { balance, height })
}

pub fn query_total_staked_at_height(
    deps: Deps,
    _env: Env,
    height: Option<u64>,
) -> StdResult<GetTotalStakedAtHeightResponse> {
    let height = height.unwrap_or(_env.block.height);
    let total = STAKED_TOTAL
        .may_load_at_height(deps.storage, height)?
        .unwrap_or_default();
    Ok(GetTotalStakedAtHeightResponse { total, height })
}

pub fn query_total_stakers_at_height(
    deps: Deps,
    _env: Env,
    height: Option<u64>,
) -> StdResult<GetTotalStakersAtHeightResponse> {
    let height = height.unwrap_or(_env.block.height);
    let total = STAKERS_TOTAL
        .may_load_at_height(deps.storage, height)?
        .unwrap_or_default();
    Ok(GetTotalStakersAtHeightResponse { total, height })
}

pub fn query_staked_value(
    deps: Deps,
    _env: Env,
    address: String,
) -> StdResult<GetStakedValueResponse> {
    let address = deps.api.addr_validate(&address)?;
    let staked = STAKED_BALANCES
        .load(deps.storage, &address)
        .unwrap_or_default();
    let total = STAKED_TOTAL.load(deps.storage)?;
    if staked == Uint128::zero() || total == Uint128::zero() {
        Ok(GetStakedValueResponse {
            value: Uint128::zero(),
        })
    } else {
        Ok(GetStakedValueResponse { value: staked })
    }
}

pub fn query_reward_amount(deps: Deps, env: Env, address: String) -> StdResult<GetRewardAmountResponse> {
    let address = deps.api.addr_validate(&address)?;
    let config = CONFIG.load(deps.storage)?;
    let prev_staked_time: Timestamp = STAKED_TIMES
        .load(deps.storage, &address)
        .unwrap_or_default();
    let user_staked_total = STAKED_BALANCES
        .load(deps.storage, &address)
        .unwrap_or_default();

    let stake_duration = env.block.time.seconds() - prev_staked_time.seconds();
    let mut reward_amount: u128 = (user_staked_total.u128()
        .checked_mul(stake_duration as u128)
        .unwrap_or(0)
        .checked_mul(config.interest_rate.u64() as u128)
        .unwrap_or(0))
        .checked_div(10000u128
            .checked_mul(365u128)
            .and_then(|v| v.checked_mul(24u128))
            .and_then(|v| v.checked_mul(60u128))
            .and_then(|v| v.checked_mul(60u128))
            .unwrap_or(1)) 
        .unwrap_or(0);
    reward_amount = reward_amount.checked_mul(config.lock_duration_multiplier.u64() as u128).and_then(|v| v.checked_div(10000u128)).unwrap_or(0);

    Ok(GetRewardAmountResponse { reward_amount: Uint128::from(reward_amount) })
}

pub fn query_staked_time(
    deps: Deps,
    env: Env,
    address: String
) -> StdResult<GetStakedTimeResponse> {
    let user_staked_time = STAKED_TIMES
        .may_load(deps.storage, &deps.api.addr_validate(&address)?)?
        .unwrap_or(env.block.time);
 
    Ok(GetStakedTimeResponse { stake_time: user_staked_time })
}

pub fn query_next_unlock_time(deps: Deps, env: Env, address: String) -> StdResult<GetNextUnlockTimeResponse> {
    let address = deps.api.addr_validate(&address)?;
    let config = CONFIG.load(deps.storage)?;
    let user_staked_time = STAKED_TIMES
        .may_load(deps.storage, &address)?
        .unwrap_or(env.block.time);
    let next_unlock_time = Timestamp::from_seconds(user_staked_time.seconds() + config.lock_duration.u64());

    Ok(GetNextUnlockTimeResponse { next_unlock_time })
}

pub fn query_total_value(deps: Deps, _env: Env) -> StdResult<GetTotalValueResponse> {
    let total_staked = STAKED_TOTAL.load(deps.storage)?;
    Ok(GetTotalValueResponse { total: total_staked })
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

pub fn query_list_stakers(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Binary> {
    let start_at = start_after
        .map(|addr| deps.api.addr_validate(&addr))
        .transpose()?;

    let stakers = cw_paginate_storage::paginate_snapshot_map(
        deps,
        &STAKED_BALANCES,
        start_at.as_ref(),
        limit,
        cosmwasm_std::Order::Ascending,
    )?;

    let stakers = stakers
        .into_iter()
        .map(|(address, balance)| GetStakerBalanceResponse {
            address: address.into_string(),
            balance,
        })
        .collect();

    to_json_binary(&ListStakersResponse { stakers })
}