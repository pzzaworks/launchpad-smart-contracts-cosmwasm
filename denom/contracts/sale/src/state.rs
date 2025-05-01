use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128, Uint64};
use cw_controllers::Hooks;
use cw_storage_plus::{Item, SnapshotItem, SnapshotMap, Strategy};

#[cw_serde]
pub struct Status {
    pub register_paused: bool,
    pub staker_paused: bool,
    pub fcfs_paused: bool,
}

impl Default for Status {
    fn default() -> Self {
        Status {
            register_paused: false,
            staker_paused: false,
            fcfs_paused: false,
        }
    }
}

#[cw_serde]
pub struct Dates {
    pub register_start: Uint64,
    pub register_end: Uint64,
    pub staker_start: Uint64,
    pub staker_end: Uint64,
    pub fcfs_start: Uint64,
    pub fcfs_end: Uint64,
}

impl Default for Dates {
    fn default() -> Self {
        Dates {
            register_start: Uint64::new(0),
            register_end: Uint64::new(0),
            staker_start: Uint64::new(0),
            staker_end: Uint64::new(0),
            fcfs_start: Uint64::new(0),
            fcfs_end: Uint64::new(0),
        }
    }
}

#[cw_serde]
pub struct WhitelistProperties {
    pub whitelist_merkle_root: String,
    pub whitelisted_user_count: Uint128,
    pub whitelisted_user_allocation: Uint128,
}

impl Default for WhitelistProperties {
    fn default() -> Self {
        WhitelistProperties {
            whitelist_merkle_root: String::new(),
            whitelisted_user_count: Uint128::zero(),
            whitelisted_user_allocation: Uint128::zero(),
        }
    }
}

#[cw_serde]
pub struct Config {
    pub owner: Addr,
    pub stake_controller: Addr,
    pub payment_denom: String,
    pub sale_token_decimals: Uint64,
    pub sale_token_price: Uint128,
    pub min_allocation: Uint128,
    pub total_allocation: Uint128,
    pub fcfs_allocation: Uint128,
    pub status: Status,
    pub dates: Dates,
    pub whitelist_properties: WhitelistProperties,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            owner: Addr::unchecked(""),
            stake_controller: Addr::unchecked(""),
            payment_denom: String::new(),
            sale_token_decimals: Uint64::zero(),
            sale_token_price: Uint128::zero(),
            min_allocation: Uint128::zero(),
            total_allocation: Uint128::zero(),
            fcfs_allocation: Uint128::zero(),
            status: Status::default(),
            dates: Dates::default(),
            whitelist_properties: WhitelistProperties::default(),
        }
    }
}

#[cw_serde]
pub struct UserInfo {
    pub address: Addr,
    pub registered: bool,
    pub tier: Uint64,
    pub total_staked: Uint128,
    pub total_payment_amount: Uint128,
    pub total_sale_token_amount: Uint128,
    pub joined_staker_round: bool,
    pub joined_fcfs_round: bool,
}

impl Default for UserInfo {
    fn default() -> Self {
        UserInfo {
            address: Addr::unchecked(""),
            registered: false,
            tier: Uint64::zero(),
            total_staked: Uint128::zero(),
            total_payment_amount: Uint128::zero(),
            total_sale_token_amount: Uint128::zero(),
            joined_staker_round: false,
            joined_fcfs_round: false,
        }
    }
}

#[cw_serde]
pub struct Statistics {
    pub total_registered_users: Uint128,
    pub total_staker_round_participants: Uint128,
    pub total_fcfs_round_participants: Uint128,
    pub total_participants: Uint128,
    pub total_staked: Uint128,
    pub total_payment_amount: Uint128,
}

impl Default for Statistics {
    fn default() -> Self {
        Statistics {
            total_registered_users: Uint128::zero(),
            total_staker_round_participants: Uint128::zero(),
            total_fcfs_round_participants: Uint128::zero(),
            total_participants: Uint128::zero(),
            total_staked: Uint128::zero(),
            total_payment_amount: Uint128::zero(),
        }
    }
}

pub const CONFIG: Item<Config> = Item::new("config");

pub const USER_INFO: SnapshotMap<&Addr, UserInfo> = SnapshotMap::new(
    "user_infos",
    "user_info__checkpoints",
    "user_info__changelog",
    Strategy::EveryBlock,
);

pub const STATISTICS: SnapshotItem<Statistics> = SnapshotItem::new(
    "statistics",
    "statistics__checkpoints",
    "statistics__changelog",
    Strategy::EveryBlock,
);

pub const BALANCE: Item<Uint128> = Item::new("balance");
pub const HOOKS: Hooks = Hooks::new("hooks");