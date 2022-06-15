use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::types::{JoinAmount, JoinType, SwapAmountWithLimit};
use crate::{Step, Swap};
use cosmwasm_std::{Coin, CosmosMsg, CustomMsg, Uint128};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]

/// A number of Custom messages that can call into the Osmosis bindings
pub enum OsmosisMsg {
    // Mint module messages
    /// CreateDenom creates a new factory denom, of denomination:
    /// factory/{creating contract bech32 address}/{Subdenom}
    /// Subdenom can be of length at most 44 characters, in [0-9a-zA-Z./]
    /// Empty subdenoms are valid.
    /// The (creating contract address, subdenom) pair must be unique.
    /// The created denom's admin is the creating contract address,
    /// but this admin can be changed using the UpdateAdmin binding.
    CreateDenom {
        subdenom: String,
    },
    /// ChangeAdmin changes the admin for a factory denom.
    /// Can only be called by the current contract admin.
    /// If the NewAdminAddress is empty, the denom will have no admin.
    ChangeAdmin {
        denom: String,
        new_admin_address: String,
    },
    /// Contracts can mint native tokens for an existing factory denom
    /// that they are the admin of.
    MintTokens {
        denom: String,
        amount: Uint128,
        mint_to_address: String,
    },
    /// Contracts can burn native tokens for an existing factory denom
    /// that they are the admin of.
    /// Currently, the burn from address must be the admin contract.
    BurnTokens {
        denom: String,
        amount: Uint128,
        burn_from_address: String,
    },
    // GAMM Module messages
    /// Swap over one or more pools
    /// Returns SwapResponse in the data field of the Response
    Swap {
        first: Swap,
        route: Vec<Step>,
        amount: SwapAmountWithLimit,
    },
    JoinPool {
        pool_id: Uint128,
        amount: JoinAmount,
    },
    ExitPool {},
    // Lockup module messages
    /// Bond gamm tokens for a duration to generate rewards
    LockTokens {
        /// Only pool tokens are valid here (e.g.: "gamm/pool/1")
        denom: String,
        amount: Uint128,
        /// The duration in seconds
        duration: String,
    },
    /// Unbond locked gamm tokens
    BeginUnlocking {
        denom: String,
        amount: Uint128,
        duration: String,
    },
    /// Unbond all locked gamm tokens
    BeginUnlockingAll {},
}

impl OsmosisMsg {
    /// Basic helper to define a swap with one pool
    pub fn simple_swap(
        pool_id: u64,
        denom_in: impl Into<String>,
        denom_out: impl Into<String>,
        amount: SwapAmountWithLimit,
    ) -> Self {
        OsmosisMsg::Swap {
            first: Swap::new(pool_id, denom_in, denom_out),
            amount,
            route: vec![],
        }
    }

    pub fn join_pool(
        pool_id: impl Into<Uint128>,
        tokens_in: Vec<Coin>,
        shares_out: impl Into<Uint128>,
        join_type: JoinType,
    ) -> Self {
        let amount = match join_type {
            JoinType::Full {} => JoinAmount::new(tokens_in, shares_out.into()),
            JoinType::SwapForExactShares {} => {
                assert!(tokens_in.len() == 1);
                JoinAmount::new_with_swap(tokens_in[0].clone(), shares_out.into(), false)
            }
            JoinType::SwapForExactTokens {} => {
                assert!(tokens_in.len() == 1);
                JoinAmount::new_with_swap(tokens_in[0].clone(), shares_out.into(), true)
            }
        };
        OsmosisMsg::JoinPool {
            pool_id: pool_id.into(),
            amount: amount.into(),
        }
    }

    pub fn lock(denom: &str, amount: impl Into<Uint128>, seconds: impl Into<Uint128>) -> Self {
        OsmosisMsg::LockTokens {
            denom: denom.to_string(),
            amount: amount.into(),
            duration: seconds.into().to_string(),
        }
    }

    pub fn mint_contract_tokens(denom: String, amount: Uint128, mint_to_address: String) -> Self {
        OsmosisMsg::MintTokens {
            denom,
            amount,
            mint_to_address,
        }
    }

    pub fn burn_contract_tokens(
        denom: String,
        amount: Uint128,
        _burn_from_address: String,
    ) -> Self {
        OsmosisMsg::BurnTokens {
            denom,
            amount,
            burn_from_address: "".to_string(), // burn_from_address is currently disabled.
        }
    }
}

impl From<OsmosisMsg> for CosmosMsg<OsmosisMsg> {
    fn from(msg: OsmosisMsg) -> CosmosMsg<OsmosisMsg> {
        CosmosMsg::Custom(msg)
    }
}

impl CustomMsg for OsmosisMsg {}
