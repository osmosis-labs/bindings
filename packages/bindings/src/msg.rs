use cosmwasm_schema::cw_serde;
use cosmwasm_std::{CosmosMsg, CustomMsg, Uint128};

use crate::types::SwapAmountWithLimit;
use crate::{Step, Swap};

/// A number of Custom messages that can call into the Osmosis bindings
#[cw_serde]
pub enum OsmosisMsg {
    /// CreateDenom creates a new factory denom, of denomination:
    /// factory/{creating contract bech32 address}/{Subdenom}
    /// Subdenom can be of length at most 44 characters, in [0-9a-zA-Z./]
    /// Empty subdenoms are valid.
    /// The (creating contract address, subdenom) pair must be unique.
    /// The created denom's admin is the creating contract address,
    /// but this admin can be changed using the UpdateAdmin binding.
    CreateDenom { subdenom: String },
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
    /// Swap over one or more pools
    /// Returns SwapResponse in the data field of the Response
    Swap {
        first: Swap,
        route: Vec<Step>,
        amount: SwapAmountWithLimit,
    },
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
