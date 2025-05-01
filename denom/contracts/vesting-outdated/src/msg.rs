use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};
use cw_ownable::cw_ownable_execute;
pub use cw_ownable::Ownership;

use crate::state::{Config, WhitelistInfo, UserInfo, Statistics};

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Option<String>,
    pub denom: String,
    pub fee_address: String,
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

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    SetVestingStart { new_start: u64 },
    UpdateDenom { new_denom: String },
    RequestRefund { tag_id: String },
    ClaimFunds { denom: String },
    ClaimVestedTokens {},
    SetWhitelist {
        tag_id: String,
        wallets: Vec<String>,
        payment_amounts: Vec<Uint128>,
        denom: String,
        token_amounts: Vec<Uint128>,
        refund_fee: Uint128,
    },
    EmergencyWithdraw { tag_id: String },
    AddHook { addr: String },
    RemoveHook { addr: String },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    GetConfig {},
    #[returns(StatisticsResponse)]
    GetStatistics {},
    #[returns(WhitelistInfoResponse)]
    GetWhitelistInfo { wallet: String },
    #[returns(VestedAmountResponse)]
    GetVestedAmount { wallet: String },
    #[returns(ClaimableAmountResponse)]
    GetClaimableAmount { wallet: String },
    #[returns(WhitelistBatchResponse)]
    GetWhitelistBatch { start: u64, limit: u32 },
    #[returns(VestingDetailsResponse)]
    GetVestingDetails {},
    #[returns(IsWhitelistedResponse)]
    IsWhitelisted { wallet: String },
    #[returns(WhitelistIndexResponse)]
    GetWhitelistIndex { wallet: String },
    #[returns(UserInfoResponse)]
    GetUserInfo { tag_id: String, wallet: String },
    #[returns(PaymentDenomResponse)]
    GetPaymentDenom { tag_id: String },
    #[returns(NextUnlockDateResponse)]
    GetNextUnlockDate {},
    #[returns(Vec<String>)]
    GetHooks {},
    #[returns(::cw_ownable::Ownership<::cosmwasm_std::Addr>)]
    Ownership {},
}

#[cw_serde]
pub enum MigrateMsg {
    FromV1 {},
}

#[cw_serde]
pub struct ConfigResponse {
    pub config: Config,
}

#[cw_serde]
pub struct StatisticsResponse {
    pub statistics: Statistics,
}

#[cw_serde]
pub struct WhitelistInfoResponse {
    pub info: WhitelistInfo,
}

#[cw_serde]
pub struct VestedAmountResponse {
    pub amount: Uint128,
}

#[cw_serde]
pub struct ClaimableAmountResponse {
    pub amount: Uint128,
}

#[cw_serde]
pub struct WhitelistBatchResponse {
    pub whitelist: Vec<(Addr, WhitelistInfo)>,
}

#[cw_serde]
pub struct VestingDetailsResponse {
    pub start: u64,
    pub cliff: u64,
    pub duration: u64,
    pub initial_unlock_percent: u16,
    pub linear_vesting_count: u16,
}

#[cw_serde]
pub struct IsWhitelistedResponse {
    pub is_whitelisted: bool,
}

#[cw_serde]
pub struct WhitelistIndexResponse {
    pub index: u64,
}

#[cw_serde]
pub struct UserInfoResponse {
    pub info: UserInfo,
}

#[cw_serde]
pub struct PaymentDenomResponse {
    pub denom: String,
}

#[cw_serde]
pub struct GetHooksResponse {
    pub hooks: Vec<String>,
}

#[cw_serde]
pub struct NextUnlockDateResponse {
    pub next_unlock_date: u64,
}