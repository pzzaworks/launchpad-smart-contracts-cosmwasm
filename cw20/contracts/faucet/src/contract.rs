#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_json, to_json_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult, Uint128, Uint64, WasmMsg
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use cw_paginate_storage::paginate_snapshot_map;
use crate::msg::{
    ExecuteMsg, GetClaimBalancesResponse, GetConfigResponse, GetLastClaimResponse, GetTotalClaimsResponse, InstantiateMsg, ListClaimersResponse, QueryMsg, ReceiveMsg
};
use crate::state::{
    ClaimerInfo, Config, NativeCoinBalance, NativeCoinConfig, TokenBalance, TokenConfig, CONFIG, HOOKS, LAST_CLAIM_TIMES, NATIVE_TOKEN_BALANCE, TOKEN_BALANCES, TOTAL_CLAIMS
};
use crate::ContractError;
use cw2::set_contract_version;

pub const CONTRACT_NAME: &str = "crates.io:faucet";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    cw_ownable::initialize_owner(deps.storage, deps.api, msg.owner.as_deref())?;
    
    let tokens: StdResult<Vec<TokenConfig>> = msg.tokens
        .into_iter()
        .map(|t| {
            Ok(TokenConfig {
                address: deps.api.addr_validate(&t.address)?.to_string(),
                amount: t.amount,
            })
        })
        .collect();

    let config = Config {
        owner: info.sender.clone(),
        tokens: tokens?, 
        native_coin: NativeCoinConfig {
            denom: msg.native_coin.denom,
            amount: msg.native_coin.amount,
        },
        claim_interval: msg.claim_interval,
    };

    CONFIG.save(deps.storage, &config)?;
    NATIVE_TOKEN_BALANCE.save(deps.storage, &Uint128::zero())?;

    for token in &config.tokens {
        TOKEN_BALANCES.save(deps.storage, &Addr::unchecked(&token.address), &Uint128::zero())?;
    }
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
        ExecuteMsg::Receive(msg) => execute_receive(deps, env, info, msg),
        ExecuteMsg::AddNativeTokens {} => execute_add_native_tokens(deps, env, info),
        ExecuteMsg::Claim {} => execute_claim(deps, env, info),
        ExecuteMsg::UpdateConfig { tokens, native_coin, claim_interval } => execute_update_config(deps, info, tokens, native_coin, claim_interval),
        ExecuteMsg::Withdraw {} => execute_withdraw(deps, info),
        ExecuteMsg::AddHook { addr } => execute_add_hook(deps, env, info, addr),
        ExecuteMsg::RemoveHook { addr } => execute_remove_hook(deps, env, info, addr),
        ExecuteMsg::UpdateOwnership(action) => execute_update_owner(deps, info, env, action),
    }
}

pub fn execute_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    wrapper: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let token_config = config.tokens.iter().find(|t| t.address == info.sender.to_string());

    if token_config.is_none() {
        return Err(ContractError::InvalidToken {});
    }

    let msg: ReceiveMsg = from_json(&wrapper.msg)?;
    let sender = deps.api.addr_validate(&wrapper.sender)?;
    
    match msg {
        ReceiveMsg::AddTokens {} => execute_add_tokens(deps, env, sender, info.sender, wrapper.amount)
    }
}

pub fn execute_add_tokens(
    deps: DepsMut,
    _env: Env,
    sender: Addr,
    token_address: Addr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    if amount.is_zero() {
        return Err(ContractError::InvalidAmount {});
    }

    TOKEN_BALANCES.update(
        deps.storage,
        &token_address,
        |balance: Option<Uint128>| -> Result<_, ContractError> {
            let current_balance = balance.unwrap_or_default();
            current_balance
                .checked_add(amount)
                .map_err(|_| ContractError::TokenBalanceOverflow {})
        },
    )?;

    Ok(Response::new()
        .add_attribute("action", "add_tokens")
        .add_attribute("from", sender)
        .add_attribute("token", token_address)
        .add_attribute("amount", amount))
}

pub fn execute_add_native_tokens(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if info.funds.is_empty() {
        return Err(ContractError::NoFunds {});
    }

    for coin in info.funds.iter() {
        if coin.denom != config.native_coin.denom {
            return Err(ContractError::InvalidDenom {});
        }
    }

    NATIVE_TOKEN_BALANCE.update(deps.storage, |balance| -> Result<_, ContractError> {
        balance
            .checked_add(info.funds[0].amount)
            .map_err(|_| ContractError::TokenBalanceOverflow {})
    })?;

    Ok(Response::new()
        .add_attribute("action", "add_native_tokens")
        .add_attribute("amount", info.funds[0].amount)
        .add_attribute("denom", info.funds[0].denom.clone()))
}

pub fn execute_claim(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let last_claim = LAST_CLAIM_TIMES.may_load(deps.storage, &info.sender)?;
    
    if let Some(last_claim_time) = last_claim {
        let current_time = env.block.time.seconds();
        let next_claim_time = last_claim_time.seconds() + config.claim_interval.u64();
        if current_time < next_claim_time {
            return Err(ContractError::ClaimTooEarly {});
        }
    }

    let mut messages: Vec<CosmosMsg> = vec![];

    for token in &config.tokens {
        let balance = TOKEN_BALANCES.load(deps.storage, &Addr::unchecked(&token.address))?;
        if balance < token.amount {
            return Err(ContractError::InsufficientFunds {});
        }

        TOKEN_BALANCES.update(
            deps.storage,
            &Addr::unchecked(&token.address),
            |bal: Option<Uint128>| -> Result<_, ContractError> {
                bal.ok_or(ContractError::TokenBalanceNotFound {})?
                    .checked_sub(token.amount)
                    .map_err(|_| ContractError::InsufficientFunds {})
            },
        )?;

        let transfer_msg = Cw20ExecuteMsg::Transfer {
            recipient: info.sender.to_string(),
            amount: token.amount,
        };

        let wasm_msg = WasmMsg::Execute {
            contract_addr: token.address.to_string(),
            msg: to_json_binary(&transfer_msg)?,
            funds: vec![],
        };
        messages.push(wasm_msg.into());
    }

    if !config.native_coin.amount.is_zero() {
        let native_balance = NATIVE_TOKEN_BALANCE.load(deps.storage)?;
        if native_balance < config.native_coin.amount {
            return Err(ContractError::InsufficientFunds {});
        }

        NATIVE_TOKEN_BALANCE.update(deps.storage, |balance| -> Result<_, ContractError> {
            balance
                .checked_sub(config.native_coin.amount)
                .map_err(|_| ContractError::InsufficientFunds {})
        })?;

        let bank_msg = cosmwasm_std::BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![cosmwasm_std::Coin {
                denom: config.native_coin.denom.clone(),
                amount: config.native_coin.amount,
            }],
        };
        messages.push(bank_msg.into());
    }

    let current_height = env.block.height;

    LAST_CLAIM_TIMES.save(deps.storage, &info.sender, &env.block.time, current_height)?;
    
    TOTAL_CLAIMS.update(deps.storage, &info.sender, current_height, |claims: Option<Uint128>| -> StdResult<Uint128> {
        Ok(claims.unwrap_or_default() + Uint128::new(1))
    })?;

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "claim")
        .add_attribute("address", info.sender.to_string()))
}

pub fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    tokens: Option<Vec<TokenConfig>>,
    native_coin: Option<NativeCoinConfig>,
    claim_interval: Option<Uint64>,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let validated_tokens = if let Some(new_tokens) = tokens {
        let mut validated = Vec::new();
        for token in new_tokens {
            let validated_addr = deps.api.addr_validate(&token.address)?;
            if token.amount.is_zero() {
                return Err(ContractError::InvalidTokenAmount {});
            }
            validated.push(TokenConfig {
                address: validated_addr.to_string(),
                amount: token.amount,
            });
        }
        Some(validated)
    } else {
        None
    };

    CONFIG.update(deps.storage, |mut config| -> Result<_, ContractError> {
        if let Some(new_tokens) = validated_tokens {
            config.tokens = new_tokens;
        }

        if let Some(new_native_coin) = native_coin {
            if new_native_coin.denom.is_empty() {
                return Err(ContractError::InvalidNativeCoinDenom {});
            }
            if new_native_coin.amount.is_zero() {
                return Err(ContractError::InvalidNativeCoinAmount {});
            }
            config.native_coin = new_native_coin;
        }

        if let Some(new_claim_interval) = claim_interval {
            if new_claim_interval.is_zero() {
                return Err(ContractError::InvalidClaimInterval {});
            }
            config.claim_interval = new_claim_interval;
        }

        Ok(config)
    })?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

pub fn execute_withdraw(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
    let config = CONFIG.load(deps.storage)?;

    let mut messages: Vec<CosmosMsg> = vec![];

    for token in &config.tokens {
        let balance = TOKEN_BALANCES.load(deps.storage, &Addr::unchecked(&token.address))?;
        if !balance.is_zero() {
            TOKEN_BALANCES.save(deps.storage, &Addr::unchecked(&token.address), &Uint128::zero())?;

            let transfer_msg = Cw20ExecuteMsg::Transfer {
                recipient: info.sender.to_string(),
                amount: balance,
            };
            let wasm_msg = WasmMsg::Execute {
                contract_addr: token.address.to_string(),
                msg: to_json_binary(&transfer_msg)?,
                funds: vec![],
            };
            messages.push(wasm_msg.into());
        }
    }

    let native_balance = NATIVE_TOKEN_BALANCE.load(deps.storage)?;
    if !native_balance.is_zero() {
        NATIVE_TOKEN_BALANCE.save(deps.storage, &Uint128::zero())?;

        let bank_msg = cosmwasm_std::BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![cosmwasm_std::Coin {
                denom: config.native_coin.denom.clone(),
                amount: native_balance,
            }],
        };
        messages.push(bank_msg.into());
    }

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "withdraw")
        .add_attribute("address", info.sender.to_string()))
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
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_json_binary(&query_config(deps)?),
        QueryMsg::GetLastClaim { address } => to_json_binary(&query_last_claim_time(deps, address)?),
        QueryMsg::GetTotalClaims { address } => to_json_binary(&query_total_claims(deps, address)?),
        QueryMsg::ListClaimers { start_after, limit } => to_json_binary(&query_list_claimers(deps, start_after, limit)?),
        QueryMsg::GetClaimBalances {} => to_json_binary(&query_claim_balances(deps, env)?),
        QueryMsg::GetTokenBalance { address } => to_json_binary(&query_token_balance(deps, address)?),
        QueryMsg::GetNativeBalance {} => to_json_binary(&query_native_balance(deps)?),
        QueryMsg::GetHooks {} => to_json_binary(&query_hooks(deps)?),
        QueryMsg::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
    }
}

fn query_config(deps: Deps) -> StdResult<GetConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(GetConfigResponse {
        tokens: config.tokens,
        native_coin: config.native_coin,
        claim_interval: config.claim_interval,
    })
}

fn query_last_claim_time(deps: Deps, address: String) -> StdResult<GetLastClaimResponse> {
    let addr = deps.api.addr_validate(&address)?;
    let last_claim = LAST_CLAIM_TIMES.may_load(deps.storage, &addr)?;
    Ok(GetLastClaimResponse {
        address,
        last_claim_time: last_claim,
    })
}

fn query_total_claims(deps: Deps, address: String) -> StdResult<GetTotalClaimsResponse> {
    let addr = deps.api.addr_validate(&address)?;
    let total_claims = TOTAL_CLAIMS.may_load(deps.storage, &addr)?.unwrap_or_default();
    Ok(GetTotalClaimsResponse {
        address,
        total_claims,
    })
}

fn query_hooks(deps: Deps) -> StdResult<crate::msg::GetHooksResponse> {
    Ok(crate::msg::GetHooksResponse {
        hooks: HOOKS.query_hooks(deps)?.hooks,
    })
}

pub fn query_list_claimers(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Binary> {
    let start_at = start_after
        .map(|addr| deps.api.addr_validate(&addr))
        .transpose()?;

    let claimers = paginate_snapshot_map(
        deps,
        &TOTAL_CLAIMS,
        start_at.as_ref(),
        limit,
        Order::Ascending,
    )?;

    let claimers = claimers
        .into_iter()
        .map(|(address, total_claims)| {
            let last_claim_time = LAST_CLAIM_TIMES.may_load(deps.storage, &address)?;
            Ok(ClaimerInfo {
                address: address.into_string(),
                last_claim_time,
                total_claims,
            })
        })
        .collect::<StdResult<Vec<_>>>()?;

    to_json_binary(&ListClaimersResponse { claimers })
}

pub fn query_claim_balances(deps: Deps, _env: Env) -> StdResult<Binary> {
    let config = CONFIG.load(deps.storage)?;
    
    let token_balances = config.tokens
        .into_iter()
        .map(|token_config| {
            let balance = TOKEN_BALANCES.load(deps.storage, &Addr::unchecked(&token_config.address))?;
            
            Ok(TokenBalance {
                address: token_config.address,
                balance,
            })
        })
        .collect::<StdResult<Vec<_>>>()?;

    let native_balance = NATIVE_TOKEN_BALANCE.load(deps.storage)?;

    let native_coin_balance = NativeCoinBalance {
        denom: config.native_coin.denom,
        balance: native_balance,
    };

    to_json_binary(&GetClaimBalancesResponse {
        tokens: token_balances,
        native_coin: native_coin_balance,
    })
}

pub fn query_token_balance(deps: Deps, address: String) -> StdResult<TokenBalance> {
    let addr = deps.api.addr_validate(&address)?;
    let balance = TOKEN_BALANCES.load(deps.storage, &addr)?;
    Ok(TokenBalance {
        address: addr.to_string(),
        balance,
    })
}

pub fn query_native_balance(deps: Deps) -> StdResult<NativeCoinBalance> {
    let config = CONFIG.load(deps.storage)?;
    let native_balance = NATIVE_TOKEN_BALANCE.load(deps.storage)?;
    Ok(NativeCoinBalance {
        denom: config.native_coin.denom,
        balance: native_balance,
    })
}