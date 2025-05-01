use cosmwasm_std::StdError;
use hex::FromHexError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),
    #[error("{0}")]
    Hex(#[from] FromHexError),
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
    #[error("Min allocation not reached")]
    MinAllocationNotReached {},
    #[error("Invalid Cw20 address")]
    InvalidCw20 {},
    #[error("Already registered")]
    AlreadyRegistered {},
    #[error("Registration is paused")]
    RegistrationPaused {},
    #[error("Registration not started")]
    RegistrationNotStarted {},
    #[error("Registration closed")]
    RegistrationClosed {},
    #[error("User is not staker")]
    NotStaker {},
    #[error("Staker round paused")]
    StakerRoundPaused {},
    #[error("Staker round not started")]
    StakerRoundNotStarted {},
    #[error("Staker round closed")]
    StakerRoundClosed {},
    #[error("User is not registered")]
    NotRegistered {},
    #[error("User has no allocation")]
    NoAllocation {},
    #[error("User has exceeded allocation")]
    ExceedUserAllocation {},
    #[error("User has exceeded total allocation")]
    ExceedTotalAllocation {},
    #[error("Fcfs round paused")]
    FcfsRoundPaused {},
    #[error("Fcfs round not started")]
    FcfsRoundNotStarted {},
    #[error("Fcfs round closed")]
    FcfsRoundClosed {},
    #[error("Wrong length")]
    WrongLength {},
    #[error("Not whitelisted")]
    WhitelistError {},
    #[error("User cannot register")]
    CannotRegister {},
}
