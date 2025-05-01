use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Timestamp, Uint128, Uint64};
use cw_controllers::Hooks;
use cw_storage_plus::{Item, SnapshotItem, SnapshotMap, Strategy};

#[cw_serde]
pub struct Config {
    pub token_address: Addr,
    pub stake_paused: bool,
    pub unstake_paused: bool,
    pub emergency_unstake_paused: bool,
    pub interest_rate: Uint64,
    pub lock_duration: Uint64,
    pub lock_duration_multiplier: Uint64,
    pub emergency_unstake_fee_percentage: Uint64,
    pub fee_address: Addr,
}

pub const CONFIG: Item<Config> = Item::new("config");

pub const STAKED_TIMES: SnapshotMap<&Addr, Timestamp> = SnapshotMap::new(
    "staked_times",
    "staked_time__checkpoints",
    "staked_time__changelog",
    Strategy::EveryBlock,
);

pub const STAKED_BALANCES: SnapshotMap<&Addr, Uint128> = SnapshotMap::new(
    "staked_balances",
    "staked_balance__checkpoints",
    "staked_balance__changelog",
    Strategy::EveryBlock,
);

pub const STAKED_TOTAL: SnapshotItem<Uint128> = SnapshotItem::new(
    "total_staked",
    "total_staked__checkpoints",
    "total_staked__changelog",
    Strategy::EveryBlock,
);

pub const STAKERS_TOTAL: SnapshotItem<Uint128> = SnapshotItem::new(
    "total_stakers",
    "total_stakers__checkpoints",
    "total_stakers__changelog",
    Strategy::EveryBlock,
);

pub const REWARD_BALANCE: Item<Uint128> = Item::new("reward_balance");
pub const HOOKS: Hooks = Hooks::new("hooks");