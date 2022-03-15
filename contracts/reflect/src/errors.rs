use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ReflectError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Permission denied: the sender is not the current owner")]
    NotCurrentOwner { expected: String, actual: String },

    #[error("Messages empty. Must reflect at least one message")]
    MessagesEmpty,

    #[error("TODO: implement")]
    NotYetImplemented,
}
