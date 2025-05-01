use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),
    #[error("{0}")]
    Ownable(#[from] cw_ownable::OwnershipError),
    #[error(transparent)]
    HookError(#[from] cw_controllers::HookError),
    #[error("{msg}")]
    QueryError { msg: String },
    #[error("Invalid amount")]
    InvalidAmount {},
    #[error("Invalid Cw20 address")]
    InvalidCw20 {},
    #[error("Claim too early")]
    ClaimTooEarly {},
    #[error("Claim limit reached")]
    ClaimLimitReached {},
    #[error("Insufficient funds in faucet")]
    InsufficientFunds {},
    #[error("Invalid token configuration")]
    InvalidTokenConfig {},
    #[error("Invalid native coin configuration")]
    InvalidNativeCoinConfig {},
    #[error("Invalid claim interval")]
    InvalidClaimInterval {},
    #[error("Invalid token")]
    InvalidToken {},
    #[error("Invalid denom")]
    InvalidDenom {},
    #[error("No funds")]
    NoFunds {},
    #[error("Overflow")]
    Overflow {},
    #[error("Invalid native coin denom")]
    InvalidNativeCoinDenom {},
    #[error("Invalid token amount")]
    InvalidTokenAmount {},
    #[error("Invalid native coin amount")]
    InvalidNativeCoinAmount {},
    #[error("Token balance not found")]
    TokenBalanceNotFound {},
    #[error("Token balance overflow")]
    TokenBalanceOverflow {},
    #[error("Unauthorized")]
    Unauthorized {},
}