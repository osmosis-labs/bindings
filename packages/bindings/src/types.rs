use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Uint128;

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, JsonSchema, Debug)]
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

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, JsonSchema, Debug)]
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

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, JsonSchema, Debug)]
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

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, JsonSchema, Debug)]
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
