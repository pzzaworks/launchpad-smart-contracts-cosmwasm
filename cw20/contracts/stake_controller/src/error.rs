use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),
    #[error(transparent)]
    Cw20Error(#[from] cw20_base::ContractError),
    #[error(transparent)]
    Ownership(#[from] cw_ownable::OwnershipError),
    #[error(transparent)]
    HookError(#[from] cw_controllers::HookError),
    #[error("Provided cw20 errored in response to TokenInfo query")]
    InvalidCw20 {},
    #[error("Invalid input length")]
    InvalidInputLength {},
}