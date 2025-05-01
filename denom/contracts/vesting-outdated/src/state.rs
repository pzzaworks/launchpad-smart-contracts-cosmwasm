use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_controllers::Hooks;
use cw_storage_plus::{Item, SnapshotItem, SnapshotMap, Strategy};

#[cw_serde]
pub struct Config {
    pub denom: String,
    pub fee_address: Addr,
    pub total_token_on_sale: Uint128,
    pub grace_period: u64,
    pub platform_fee: Uint128,
    pub decimals: u8,
    pub start: u64,
    pub cliff: u64,
    pub duration: u64,
    pub initial_unlock_percent: u16,
    pub linear_vesting_count: u16,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            denom: "".to_string(),
            fee_address: Addr::unchecked(""),
            total_token_on_sale: Uint128::zero(),
            grace_period: 0,
            platform_fee: Uint128::zero(),
            decimals: 0,
            start: 0,
            cliff: 0,
            duration: 0,
            initial_unlock_percent: 0,
            linear_vesting_count: 0,
        }
    }
}

#[cw_serde]
pub struct WhitelistInfo {
    pub wallet: Addr,
    pub join_date: u64,
    pub amount: Uint128,
    pub distributed_amount: Uint128,
}

impl Default for WhitelistInfo {
    fn default() -> Self {
        WhitelistInfo {
            wallet: Addr::unchecked(""),
            join_date: 0,
            amount: Uint128::zero(),
            distributed_amount: Uint128::zero(),
        }
    }
}

#[cw_serde]
pub struct UserInfo {
    pub refunded: bool,
    pub refund_date: Option<u64>,
    pub payment_amount: Uint128,
    pub token_amount: Uint128,
    pub refund_fee: Uint128,
}

impl Default for UserInfo {
    fn default() -> Self {
        UserInfo {
            refunded: false,
            refund_date: None,
            payment_amount: Uint128::zero(),
            token_amount: Uint128::zero(),
            refund_fee: Uint128::zero(),
        }
    }
}

#[cw_serde]
pub struct Statistics {
    pub total_vested_token: Uint128,
    pub total_returned_token: Uint128,
}

impl Default for Statistics {
    fn default() -> Self {
        Statistics {
            total_vested_token: Uint128::zero(),
            total_returned_token: Uint128::zero(),
        }
    }
}

pub const CONFIG: Item<Config> = Item::new("config");

pub const STATISTICS: SnapshotItem<Statistics> = SnapshotItem::new(
    "statistics",
    "statistics__checkpoints",
    "statistics__changelog",
    Strategy::EveryBlock,
);

pub const WHITELIST_POOL: SnapshotMap<&Addr, WhitelistInfo> = SnapshotMap::new(
    "whitelist_pool",
    "whitelist_pool__checkpoints",
    "whitelist_pool__changelog",
    Strategy::EveryBlock,
);

pub const WHITELIST_INDEX: SnapshotMap<&Addr, u64> = SnapshotMap::new(
    "whitelist_index",
    "whitelist_index__checkpoints",
    "whitelist_index__changelog",
    Strategy::EveryBlock,
);

pub const IS_WHITELISTED: SnapshotMap<&Addr, bool> = SnapshotMap::new(
    "is_whitelisted",
    "is_whitelisted__checkpoints",
    "is_whitelisted__changelog",
    Strategy::EveryBlock,
);

pub const USER_INFO: SnapshotMap<(&str, &Addr), UserInfo> = SnapshotMap::new(
    "user_info",
    "user_info__checkpoints",
    "user_info__changelog",
    Strategy::EveryBlock,
);

pub const PAYMENT_DENOM: SnapshotMap<&str, String> = SnapshotMap::new(
    "payment_denom",
    "payment_denom__checkpoints",
    "payment_denom__changelog",
    Strategy::EveryBlock,
);

pub const HOOKS: Hooks = Hooks::new("hooks");