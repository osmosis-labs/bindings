use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Coin, Uint128};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Swap {
    pub pool_id: u64,
    pub denom_in: String,
    pub denom_out: String,
}

impl Swap {
    pub fn new(pool_id: u64, denom_in: impl Into<String>, denom_out: impl Into<String>) -> Self {
        Swap {
            pool_id,
            denom_in: denom_in.into(),
            denom_out: denom_out.into(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Step {
    pub pool_id: u64,
    pub denom_out: String,
}

impl Step {
    pub fn new(pool_id: u64, denom_out: impl Into<String>) -> Self {
        Step {
            pool_id,
            denom_out: denom_out.into(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum SwapAmount {
    In(Uint128),
    Out(Uint128),
}

impl SwapAmount {
    pub fn as_in(&self) -> Uint128 {
        match self {
            SwapAmount::In(x) => *x,
            _ => panic!("was output"),
        }
    }

    pub fn as_out(&self) -> Uint128 {
        match self {
            SwapAmount::Out(x) => *x,
            _ => panic!("was input"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum SwapAmountWithLimit {
    ExactIn { input: Uint128, min_output: Uint128 },
    ExactOut { output: Uint128, max_input: Uint128 },
}

impl SwapAmountWithLimit {
    pub fn discard_limit(self) -> SwapAmount {
        match self {
            SwapAmountWithLimit::ExactIn { input, .. } => SwapAmount::In(input),
            SwapAmountWithLimit::ExactOut { output, .. } => SwapAmount::Out(output),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct LockTokensResponse {
    pub id: u64,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum JoinAmount {
    /// Joins a pool with a maximum amount of provided tokens. This will never execute a swap.
    ExactSharesForTokenList {
        shares_out: Uint128,
        /// If this is set to true the provided token will be swapped to join the pool.
        /// Otherise two tokens will be required.
        max_tokens_in: Vec<Coin>,
    },

    /// Joins a pool with a maximum amount of one provided token and an exact expected amount
    /// of pool shares out. This will always execute a swap to join a pool.
    ExactSharesForToken {
        shares_out: Uint128,
        /// If this is set to true the provided token will be swapped to join the pool.
        /// Otherise two tokens will be required.
        max_token_in: Coin,
    },

    /// Joins a pool with an exact amount of a token and a minimum expected amount of pool shares
    /// out. This will always execute a swap to join a pool.
    ExactTokenForShares {
        /// Provided token. WIll be swapped to enter the pool
        exact_token_in: Coin,
        /// Minimum expected shares of the pool as a result
        min_shares_out: Uint128,
    },
}

impl JoinAmount {
    /// New join amount with swap
    pub fn new_with_swap(token_in: Coin, shares_out: Uint128, exact_tokens: bool) -> Self {
        match exact_tokens {
            true => JoinAmount::ExactTokenForShares {
                exact_token_in: token_in,
                min_shares_out: shares_out,
            },
            false => JoinAmount::ExactSharesForToken {
                max_token_in: token_in,
                shares_out: shares_out,
            },
        }
    }
    /// New join amount with all tokens provided
    pub fn new(tokens_in: Vec<Coin>, shares_out: Uint128) -> Self {
        assert!(tokens_in.len() > 1);
        JoinAmount::ExactSharesForTokenList {
            shares_out,
            max_tokens_in: tokens_in,
        }
    }
}
