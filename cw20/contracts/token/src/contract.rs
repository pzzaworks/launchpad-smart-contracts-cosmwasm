use cosmwasm_std::{
    Deps, DepsMut, Env, MessageInfo, Response, StdResult, Binary, to_json_binary, entry_point
};
use cw2::set_contract_version;
use cw20_base::allowances::{
    execute_transfer_from, execute_send_from, execute_burn_from,
    execute_increase_allowance, execute_decrease_allowance, query_allowance
};
use cw20_base::contract::{
    execute_transfer, execute_burn, execute_send, execute_mint, execute_update_marketing,
    execute_upload_logo, execute_update_minter, query_balance, query_token_info, query_minter, 
    query_marketing_info, query_download_logo
};
use cw20_base::ContractError;
use cw20_base::enumerable::{query_owner_allowances, query_all_accounts, query_spender_allowances};
use cw20_base::msg::{
    InstantiateMsg, ExecuteMsg, QueryMsg, MigrateMsg
};

const CONTRACT_NAME: &str = "crates.io::token";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps : DepsMut,
    env  : Env,
    info : MessageInfo,
    msg  : InstantiateMsg
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    cw20_base::contract::instantiate(deps, env, info, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps : DepsMut,
    env  : Env,
    info : MessageInfo,
    msg  : ExecuteMsg
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Transfer {
            recipient,
            amount
        } => execute_transfer(deps, env, info, recipient, amount),

        ExecuteMsg::Burn {
            amount
        } => execute_burn(deps, env, info, amount),

        ExecuteMsg::Send {
            contract,
            amount,
            msg
        } => execute_send(deps, env, info, contract, amount, msg),

        ExecuteMsg::IncreaseAllowance {
            spender,
            amount, 
            expires 
        } => execute_increase_allowance(deps, env, info, spender, amount, expires),
        
        ExecuteMsg::DecreaseAllowance { 
            spender, 
            amount, 
            expires 
        } => execute_decrease_allowance(deps, env, info, spender, amount, expires),

        ExecuteMsg::TransferFrom {
            owner,
            recipient,
            amount
        } => execute_transfer_from(deps, env, info, owner, recipient, amount),

        ExecuteMsg::SendFrom {
            owner,
            contract,
            amount,
            msg 
        } => execute_send_from(deps, env, info, owner, contract, amount, msg),

        ExecuteMsg::BurnFrom { 
            owner, 
            amount 
        } => execute_burn_from(deps, env, info, owner, amount),

        ExecuteMsg::Mint { 
            recipient, 
            amount 
        } => execute_mint(deps, env, info, recipient, amount),

        ExecuteMsg::UpdateMinter {
            new_minter
        } => execute_update_minter(deps, env, info, new_minter),

        ExecuteMsg::UpdateMarketing {
            project,
            description,
            marketing
        } => execute_update_marketing(deps, env, info, project, description, marketing),

        ExecuteMsg::UploadLogo(logo) => execute_upload_logo(deps, env, info, logo),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(
    deps: Deps,
    _env: Env,
    msg : QueryMsg
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Balance { 
            address 
        } => to_json_binary(&query_balance(deps, address)?),
        
        QueryMsg::TokenInfo {
        } => to_json_binary(&query_token_info(deps)?),

        QueryMsg::Minter {
        } => to_json_binary(&query_minter(deps)?),

        QueryMsg::Allowance {
            owner,
            spender
        } => to_json_binary(&query_allowance(deps, owner, spender)?),

        QueryMsg::AllAllowances {
            owner,
            start_after,
            limit
        } => to_json_binary(&query_owner_allowances(deps, owner, start_after, limit)?),

        QueryMsg::AllSpenderAllowances {
            spender,
            start_after,
            limit
        } => to_json_binary(&query_spender_allowances(deps, spender, start_after, limit)?),

        QueryMsg::AllAccounts {
            start_after,
            limit
        } => to_json_binary(&query_all_accounts(deps, start_after, limit)?),

        QueryMsg::MarketingInfo {
        } => to_json_binary(&query_marketing_info(deps)?),

        QueryMsg::DownloadLogo {
        } => to_json_binary(&query_download_logo(deps)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    cw20_base::contract::migrate(deps, _env, _msg)
}
