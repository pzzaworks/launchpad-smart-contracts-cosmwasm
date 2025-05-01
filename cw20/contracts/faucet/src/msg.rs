use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Timestamp, Uint128, Uint64};
use cw20::Cw20ReceiveMsg;
use cw_ownable::cw_ownable_execute;
pub use cw_ownable::Ownership;

use crate::state::{ClaimerInfo, NativeCoinBalance, NativeCoinConfig, TokenBalance, TokenConfig};

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Option<String>,
    pub tokens: Vec<TokenConfig>,
    pub native_coin: NativeCoinConfig,
    pub claim_interval: Uint64,
}

#[cw_serde]
pub enum ReceiveMsg {
    AddTokens {}
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    AddNativeTokens {},
    Claim {},
    UpdateConfig { 
        tokens: Option<Vec<TokenConfig>>,
        native_coin: Option<NativeCoinConfig>,
        claim_interval: Option<Uint64>,
    },
    Withdraw {},
    AddHook { addr: String },
    RemoveHook { addr: String },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GetLastClaimResponse)]
    GetLastClaim { address: String },
    #[returns(GetTotalClaimsResponse)]
    GetTotalClaims { address: String },
    #[returns(GetConfigResponse)]
    GetConfig {},
    #[returns(GetHooksResponse)]
    GetHooks {},
    #[returns(ListClaimersResponse)]
    ListClaimers {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(::cw_ownable::Ownership<::cosmwasm_std::Addr>)]
    Ownership {},
    #[returns(GetClaimBalancesResponse)]
    GetClaimBalances {},
    #[returns(GetTokenBalanceResponse)]
    GetTokenBalance { address: String },
    #[returns(GetNativeBalanceResponse)]
    GetNativeBalance {},
}

#[cw_serde]
pub struct GetLastClaimResponse {
    pub address: String,
    pub last_claim_time: Option<Timestamp>,
}

#[cw_serde]
pub struct GetTotalClaimsResponse {
    pub address: String,
    pub total_claims: Uint128,
}

#[cw_serde]
pub struct GetConfigResponse {
    pub tokens: Vec<TokenConfig>,
    pub native_coin: NativeCoinConfig,
    pub claim_interval: Uint64,
}

#[cw_serde]
pub struct GetHooksResponse {
    pub hooks: Vec<String>,
}

#[cw_serde]
pub struct ListClaimersResponse {
    pub claimers: Vec<ClaimerInfo>,
}

#[cw_serde]
pub struct GetClaimBalancesResponse {
    pub tokens: Vec<TokenBalance>,
    pub native_coin: NativeCoinBalance,
}

#[cw_serde]
pub struct GetTokenBalanceResponse {
    pub address: String,
    pub balance: Uint128,
}

#[cw_serde]
pub struct GetNativeBalanceResponse {
    pub native_coin: NativeCoinBalance,
}