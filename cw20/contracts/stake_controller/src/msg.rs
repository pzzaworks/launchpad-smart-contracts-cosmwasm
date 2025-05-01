use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Uint128, Uint64};
use cw_ownable::cw_ownable_execute;
pub use cw_ownable::Ownership;

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Option<String>,
    pub token_address: String,
    pub stake_contracts: Vec<String>,
    pub stake_contract_multipliers: Vec<Uint64>,
    pub tier_thresholds: Vec<Uint128>,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    UpdateConfig { stake_contracts: Vec<String>, stake_contract_multipliers: Vec<Uint64>, tier_thresholds: Vec<Uint128> },
    UpdateCustomTiers { address: String, tier_index: Uint64 },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(crate::state::Config)]
    GetConfig {},
    #[returns(::cw_ownable::Ownership::<::cosmwasm_std::Addr>)]
    Ownership {},
    #[returns(GetUserTierResponse)]
    GetUserTierIndex { address: String },
    #[returns(Uint128)]
    GetTotalStaked { address: String },
    #[returns(GetStakedValueResponse)]
    GetStakedValue { address: String },
    #[returns(Uint64)]
    GetUserCustomTier { address: String },
}

#[cw_serde]
pub enum MigrateMsg {
    FromV1 {},
}

#[cw_serde]
pub struct GetUserTierResponse {
    pub tier: Uint64,
    pub total_staked: Uint128,
}

#[cw_serde]
pub struct GetStakedValueResponse {
    pub value: Uint128,
}
