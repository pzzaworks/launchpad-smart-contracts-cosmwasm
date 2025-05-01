use cosmwasm_std::{Addr, StdError};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),
    #[error(transparent)]
    Cw20Error(#[from] cw20_base::ContractError),
    #[error(transparent)]
    Ownership(#[from] cw_ownable::OwnershipError),
    #[error(transparent)]
    HookError(#[from] cw_controllers::HookError),
    #[error("{msg}")]
    QueryError { msg: String },
    #[error("Invalid amount")]
    InvalidAmount {},
    #[error("Stake paused. Cannot stake")]
    StakePaused {},
    #[error("Unstake paused. Cannot unstake")]
    UnstakePaused {},
    #[error("Emergency unstake paused. Cannot unstake")]
    EmergencyUnstakePaused {},
    #[error("Provided cw20 errored in response to TokenInfo query")]
    InvalidCw20 {},
    #[error("Nothing to unstake")]
    NothingStaked {},
    #[error("Unstaking this amount violates the invariant: (cw20 total_supply <= 2^128)")]
    Cw20InvariantViolation {},
    #[error("Cannot unstake more than has been staked")]
    ImpossibleUnstake {},
    #[error("Invalid lock duration, lock duration cannot be 0")]
    InvalidLockDuration {},
    #[error("Invalid token")]
    InvalidToken { received: Addr, expected: Addr },
    #[error("Lock duration not passed")]
    LockDurationNotPassed {},
    #[error("Invalid interest rate, interest rate cannot be 0")]
    InvalidInterestRate {},
    #[error("No rewards to harvest")]
    NoRewardsToHarvest {},
    #[error("No rewards to re-invest")]
    NoRewardsToReInvest {},
    #[error("Insufficient funds")]
    InsufficientFunds {},
    #[error("Insufficient reward balance")]
    InsufficientRewardBalance {},
}
