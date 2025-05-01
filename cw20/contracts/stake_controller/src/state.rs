use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128, Uint64};
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Config {
    pub token_address: Addr,
    pub stake_contracts: Vec<Addr>,
    pub stake_contract_multipliers: Vec<Uint64>,
    pub tier_thresholds: Vec<Uint128>,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const CUSTOM_TIERS: Map<&Addr, Uint64> = Map::new("custom_tiers");