use crate::state::HOOKS;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_json_binary, Addr, StdResult, Storage, SubMsg, Uint128, WasmMsg};

#[cw_serde]
pub enum VestingHookMsg {
    Whitelist { addr: Addr, amount: Uint128 },
    Claim { addr: Addr, amount: Uint128 },
    Refund { addr: Addr, amount: Uint128 },
}

pub fn whitelist_hook_msgs(
    storage: &dyn Storage,
    addr: Addr,
    amount: Uint128,
) -> StdResult<Vec<SubMsg>> {
    let msg = to_json_binary(&VestingExecuteMsg::VestingHook(
        VestingHookMsg::Whitelist { addr, amount },
    ))?;
    HOOKS.prepare_hooks(storage, |a| {
        let execute = WasmMsg::Execute {
            contract_addr: a.to_string(),
            msg: msg.clone(),
            funds: vec![],
        };
        Ok(SubMsg::new(execute))
    })
}

pub fn claim_hook_msgs(
    storage: &dyn Storage,
    addr: Addr,
    amount: Uint128,
) -> StdResult<Vec<SubMsg>> {
    let msg = to_json_binary(&VestingExecuteMsg::VestingHook(
        VestingHookMsg::Claim { addr, amount },
    ))?;
    HOOKS.prepare_hooks(storage, |a| {
        let execute = WasmMsg::Execute {
            contract_addr: a.to_string(),
            msg: msg.clone(),
            funds: vec![],
        };
        Ok(SubMsg::new(execute))
    })
}

pub fn refund_hook_msgs(
    storage: &dyn Storage,
    addr: Addr,
    amount: Uint128,
) -> StdResult<Vec<SubMsg>> {
    let msg = to_json_binary(&VestingExecuteMsg::VestingHook(
        VestingHookMsg::Refund { addr, amount },
    ))?;
    HOOKS.prepare_hooks(storage, |a| {
        let execute = WasmMsg::Execute {
            contract_addr: a.to_string(),
            msg: msg.clone(),
            funds: vec![],
        };
        Ok(SubMsg::new(execute))
    })
}

#[cw_serde]
enum VestingExecuteMsg {
    VestingHook(VestingHookMsg),
}