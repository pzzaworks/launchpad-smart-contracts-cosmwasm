use cosmwasm_std::StdError;
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
    #[error("Unauthorized")]
    Unauthorized {},
    #[error("Vesting already started")]
    VestingAlreadyStarted {},
    #[error("Invalid token")]
    InvalidToken {},
    #[error("Not in grace period")]
    NotInGracePeriod {},
    #[error("User already refunded")]
    UserAlreadyRefunded {},
    #[error("User already claimed")]
    UserAlreadyClaimed {},
    #[error("Grace period in progress")]
    GracePeriodInProgress {},
    #[error("No tokens to claim")]
    NoTokensToClaim {},
    #[error("Mismatched array lengths")]
    MismatchedArrayLengths {},
    #[error("Custom Error val: {val:?}")]
    CustomError { val: String },
    #[error("Invalid vesting schedule")]
    InvalidVestingSchedule {},
    #[error("Insufficient funds")]
    InsufficientFunds {},
    #[error("Vesting not started")]
    VestingNotStarted {},
    #[error("Vesting period ended")]
    VestingPeriodEnded {},
    #[error("Invalid whitelist entry")]
    InvalidWhitelistEntry {},
    #[error("User not whitelisted")]
    UserNotWhitelisted {},
    #[error("Amount exceeds allowance")]
    AmountExceedsAllowance {},
    #[error("Invalid payment token")]
    InvalidPaymentToken {},
    #[error("Refund period ended")]
    RefundPeriodEnded {},
    #[error("Invalid refund fee")]
    InvalidRefundFee {},
    #[error("Contract paused")]
    ContractPaused {},
    #[error("Invalid claim amount")]
    InvalidClaimAmount {},
    #[error("Hook already exists")]
    HookAlreadyExists {},
    #[error("Hook does not exist")]
    HookDoesNotExist {},
    #[error("No funds to claim")]
    NoFundsToClaim {},
    #[error("Cliff period not ended")]
    CliffPeriodNotEnded {},
}