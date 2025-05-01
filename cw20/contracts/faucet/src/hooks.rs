use crate::state::HOOKS;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_json_binary, Addr, StdResult, Storage, SubMsg, Uint128, WasmMsg};

#[cw_serde]
pub enum FaucetHookMsg {
    Claim { addr: Addr, tokens: Vec<TokenClaim>, native_coin: Option<NativeCoinClaim> },
}

#[cw_serde]
pub struct TokenClaim {
    pub address: String,
    pub amount: Uint128,
}

#[cw_serde]
pub struct NativeCoinClaim {
    pub denom: String,
    pub amount: Uint128,
}

pub fn claim_hook_msgs(
    storage: &dyn Storage,
    addr: Addr,
    tokens: Vec<TokenClaim>,
    native_coin: Option<NativeCoinClaim>,
) -> StdResult<Vec<SubMsg>> {
    let msg = to_json_binary(&FaucetExecuteMsg::FaucetHook(
        FaucetHookMsg::Claim { addr, tokens, native_coin },
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
enum FaucetExecuteMsg {
    FaucetHook(FaucetHookMsg),
}