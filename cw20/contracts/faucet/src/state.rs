use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Timestamp, Uint128, Uint64};
use cw_controllers::Hooks;
use cw_storage_plus::{Item, Map, SnapshotMap, Strategy};

#[cw_serde]
pub struct TokenConfig {
    pub address: String,
    pub amount: Uint128,
}

impl Default for TokenConfig {
    fn default() -> Self {
        TokenConfig {
            address: "".to_string(),
            amount: Uint128::zero(),
        }
    }
}

#[cw_serde]
pub struct NativeCoinConfig {
    pub denom: String,
    pub amount: Uint128,
}

impl Default for NativeCoinConfig {
    fn default() -> Self {
        NativeCoinConfig {
            denom: "".to_string(),
            amount: Uint128::zero(),
        }
    }
}

#[cw_serde]
pub struct Config {
    pub owner: Addr,
    pub tokens: Vec<TokenConfig>,
    pub native_coin: NativeCoinConfig,
    pub claim_interval: Uint64,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            owner: Addr::unchecked(""),
            tokens: vec![],
            native_coin: NativeCoinConfig::default(),
            claim_interval: Uint64::zero(),
        }
    }
}

#[cw_serde]
pub struct ClaimerInfo {
    pub address: String,
    pub last_claim_time: Option<Timestamp>,
    pub total_claims: Uint128,
}

impl Default for ClaimerInfo {
    fn default() -> Self {
        ClaimerInfo {
            address: "".to_string(),
            last_claim_time: None,
            total_claims: Uint128::zero(),
        }
    }
    
}

#[cw_serde]
pub struct TokenBalance {
    pub address: String,
    pub balance: Uint128,
}

impl Default for TokenBalance {
    fn default() -> Self {
        TokenBalance {
            address: "".to_string(),
            balance: Uint128::zero(),
        }
    }
}

#[cw_serde]
pub struct NativeCoinBalance {
    pub denom: String,
    pub balance: Uint128,
}

impl Default for NativeCoinBalance {
    fn default() -> Self {
        NativeCoinBalance {
            denom: "".to_string(),
            balance: Uint128::zero(),
        }
    }
}

pub const CONFIG: Item<Config> = Item::new("config");

pub const LAST_CLAIM_TIMES: SnapshotMap<&Addr, Timestamp> = SnapshotMap::new(
    "last_claim_times",
    "last_claim_times__checkpoints",
    "last_claim_times__changelog",
    Strategy::EveryBlock,
);

pub const TOTAL_CLAIMS: SnapshotMap<&Addr, Uint128> = SnapshotMap::new(
    "total_claims",
    "total_claims__checkpoint",
    "total_claims__changelog",
    Strategy::EveryBlock,
);

pub const TOKEN_BALANCES: Map<&Addr, Uint128> = Map::new("token_balances");
pub const NATIVE_TOKEN_BALANCE: Item<Uint128> = Item::new("native_token_balance");
pub const HOOKS: Hooks = Hooks::new("hooks");