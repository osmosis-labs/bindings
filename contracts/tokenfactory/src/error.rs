use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum TokenFactoryError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Invalid subdenom: {subdenom:?}")]
    InvalidSubdenom { subdenom: String },

    #[error("Invalid denom: {denom:?} {message:?}")]
    InvalidDenom { denom: String, message: String },

    #[error("Burn from address is not supported yet, was: {address:?}")]
    BurnTokensFromAddressNotSupported { address: String },

    #[error("Burn amount was zero, must be positive")]
    BurnTokensZeroBurnAmount {},
}
