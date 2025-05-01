use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Uint128, Uint64};
use cw20::Cw20ReceiveMsg;
use cw_ownable::cw_ownable_execute;
pub use cw_ownable::Ownership;
use crate::state::{Dates, Statistics, Status, UserInfo, WhitelistProperties};

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Option<String>,
    pub stake_controller: String,
    pub payment_token: String,
    pub sale_token_decimals: Uint64,
    pub sale_token_price: Uint128,
    pub min_allocation: Uint128,
    pub total_allocation: Uint128,
    pub fcfs_multiplier: Uint64,
    pub fcfs_allocation: Uint128,
    pub status: Status,
    pub dates: Dates,
    pub whitelist_properties: WhitelistProperties,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    Register { proof: Option<Vec<String>> },
    Receive(Cw20ReceiveMsg),
    UpdateConfig { stake_controller: String, payment_token: String, sale_token_decimals: Uint64, sale_token_price: Uint128, min_allocation: Uint128, total_allocation: Uint128, fcfs_multiplier: Uint64, status: Status, dates: Dates, whitelist_properties: WhitelistProperties },
    Withdraw {},
    AddHook { addr: String },
    RemoveHook { addr: String },
}

#[cw_serde]
pub enum ReceiveMsg {
    JoinStakerRound { proof: Option<Vec<String>> },
    JoinFcfsRound {}
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(crate::state::Config)]
    GetConfig {},
    #[returns(GetHooksResponse)]
    GetHooks {},
    #[returns(::cw_ownable::Ownership::<::cosmwasm_std::Addr>)]
    Ownership {},
    #[returns(GetUserTierResponse)]
    GetUserTierIndex { address: String },
    #[returns(GetBalanceResponse)]
    GetBalance {},
    #[returns(GetStatisticsResponse)]
    GetStatistics { height: Option<u64> },
    #[returns(GetUserInfoAtHeightResponse)]
    GetUserInfoAtHeight { address: String, height: Option<u64> },
    #[returns(GetAllUserInfoAtHeightResponse)]
    GetAllUserInfoAtHeight { height: Option<u64> },
    #[returns(Uint128)]
    GetUserStakerAllocation { address: String, proof: Option<Vec<String>> },
    #[returns(Uint128)]
    GetUserFCFSAllocation {},
    #[returns(bool)]
    VerifyProof { address: String, proof: Vec<String> },
}

#[cw_serde]
pub enum MigrateMsg {
    FromV1 {},
}

#[cw_serde]
pub struct GetHooksResponse {
    pub hooks: Vec<String>,
}

#[cw_serde]
pub struct GetUserTierResponse {
    pub tier: Uint64,
    pub total_staked: Uint128,
}

#[cw_serde]
pub struct GetBalanceResponse {
    pub balance: Uint128,
}

#[cw_serde]
pub struct GetUserInfoAtHeightResponse {
    pub user_info: UserInfo,
    pub height: Uint64,
}

#[cw_serde]
pub struct GetAllUserInfoAtHeightResponse {
    pub user_infos: Vec<UserInfo>,
    pub height: Uint64,
}

#[cw_serde]
pub struct GetStatisticsResponse {
    pub statistics: Statistics,
    pub height: Uint64,
}