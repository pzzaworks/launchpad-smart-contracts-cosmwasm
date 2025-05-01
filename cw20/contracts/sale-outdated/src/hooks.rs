use crate::state::HOOKS;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_json_binary, Addr, StdResult, Storage, SubMsg, Uint128, Uint64, WasmMsg};

#[cw_serde]
pub enum SaleHookMsg {
    Register { addr: Addr, tier_id: Uint64, total_staked: Uint128 },
    JoinStakerRound { addr: Addr, amount: Uint128 },
    JoinFcfsRound { addr: Addr, amount: Uint128 },
}

pub fn register_hook_msgs(
    storage: &dyn Storage,
    addr: Addr,
    tier_id: Uint64,
    total_staked: Uint128,
) -> StdResult<Vec<SubMsg>> {
    let msg = to_json_binary(&SaleExecuteMsg::SaleHook(
        SaleHookMsg::Register { addr, tier_id, total_staked },
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

pub fn join_staker_round_hook_msgs(
    storage: &dyn Storage,
    addr: Addr,
    amount: Uint128,
) -> StdResult<Vec<SubMsg>> {
    let msg = to_json_binary(&SaleExecuteMsg::SaleHook(
        SaleHookMsg::JoinStakerRound { addr, amount },
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

pub fn join_fcfs_round_hook_msgs(
    storage: &dyn Storage,
    addr: Addr,
    amount: Uint128,
) -> StdResult<Vec<SubMsg>> {
    let msg = to_json_binary(&SaleExecuteMsg::SaleHook(
        SaleHookMsg::JoinStakerRound { addr, amount },
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
enum SaleExecuteMsg {
    SaleHook(SaleHookMsg),
}