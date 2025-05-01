use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Timestamp, Uint128, Uint64};
use cw20::Cw20ReceiveMsg;
use cw_ownable::cw_ownable_execute;
pub use cw_ownable::Ownership;

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Option<String>,
    pub token_address: String,
    pub stake_paused: bool,
    pub unstake_paused: bool,
    pub emergency_unstake_paused: bool,
    pub interest_rate: Uint64,
    pub lock_duration: Uint64,
    pub lock_duration_multiplier: Uint64,
    pub emergency_unstake_fee_percentage: Uint64,
    pub fee_address: String,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    Unstake { amount: Uint128 },
    EmergencyUnstake { amount: Uint128 },
    Harvest {},
    ReInvest {},
    UpdateConfig { 
        stake_paused: bool, 
        unstake_paused: bool, 
        emergency_unstake_paused: bool, 
        interest_rate: Uint64, 
        lock_duration: Uint64, 
        lock_duration_multiplier: Uint64, 
        emergency_unstake_fee_percentage: Uint64,
        fee_address: String
    },
    Withdraw {},
    AddHook { addr: String },
    RemoveHook { addr: String },
}

#[cw_serde]
pub enum ReceiveMsg {
    Stake {},
    AddTokens {}
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GetStakedBalanceAtHeightResponse)]
    GetStakedBalanceAtHeight {
        address: String,
        height: Option<u64>,
    },
    #[returns(GetTotalStakedAtHeightResponse)]
    GetTotalStakedAtHeight { height: Option<u64> },
    #[returns(GetStakedValueResponse)]
    GetStakedValue { address: String },
    #[returns(GetStakedTimeResponse)]
    GetStakedTime { address: String },
    #[returns(GetRewardAmountResponse)]
    GetRewardAmount { address: String },
    #[returns(GetTotalValueResponse)]
    GetTotalValue {},
    #[returns(crate::state::Config)]
    GetConfig {},
    #[returns(GetHooksResponse)]
    GetHooks {},
    #[returns(ListStakersResponse)]
    ListStakers {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(::cw_ownable::Ownership<::cosmwasm_std::Addr>)]
    Ownership {},
    #[returns(GetTotalStakersAtHeightResponse)]
    GetTotalStakersAtHeight { height: Option<u64> },
    #[returns(GetNextUnlockTimeResponse)]
    GetNextUnlockTime { address: String },
    #[returns(GetRewardBalanceResponse)]
    GetRewardBalance {}
}

#[cw_serde]
pub enum MigrateMsg {
    FromV1 {},
}

#[cw_serde]
pub struct GetStakedBalanceAtHeightResponse {
    pub balance: Uint128,
    pub height: u64,
}

#[cw_serde]
pub struct GetTotalStakedAtHeightResponse {
    pub total: Uint128,
    pub height: u64,
}

#[cw_serde]
pub struct GetTotalStakersAtHeightResponse {
    pub total: Uint128,
    pub height: u64,
}

#[cw_serde]
pub struct GetStakedValueResponse {
    pub value: Uint128,
}

#[cw_serde]
pub struct GetStakedTimeResponse {
    pub stake_time: Timestamp,
}

#[cw_serde]
pub struct GetRewardAmountResponse {
    pub reward_amount: Uint128,
}

#[cw_serde]
pub struct GetNextUnlockTimeResponse {
    pub next_unlock_time: Timestamp,
}

#[cw_serde]
pub struct GetTotalValueResponse {
    pub total: Uint128,
}

#[cw_serde]
pub struct GetHooksResponse {
    pub hooks: Vec<String>,
}

#[cw_serde]
pub struct ListStakersResponse {
    pub stakers: Vec<GetStakerBalanceResponse>,
}

#[cw_serde]
pub struct GetStakerBalanceResponse {
    pub address: String,
    pub balance: Uint128,
}

#[cw_serde]
pub struct GetRewardBalanceResponse {
    pub reward_balance: Uint128,
}