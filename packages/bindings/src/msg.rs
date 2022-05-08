use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::types::SwapAmountWithLimit;
use crate::{Step, Swap};
use cosmwasm_std::{CosmosMsg, CustomMsg, Uint128};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
/// A number of Custom messages that can call into the Osmosis bindings
pub enum OsmosisMsg {
    /// Contracts can mint native tokens that have an auto-generated denom
    /// namespaced under the contract's address. A contract may create any number
    /// of independent sub-denoms.
    /// Returns FullDenomResponse in the data field of the Response
    MintTokens {
        /// sub_denoms (nonces in Osmosis) are validated as part of the full denomination.
        /// Can be up to 128 - prefix length (currently 7) - bech32 address length (4 (osmo) + 39) - number of separators (2) =
        /// 76 "alphanumeric" (https://github.com/cosmos/cosmos-sdk/blob/2646b474c7beb0c93d4fafd395ef345f41afc251/types/coin.go#L677)
        /// characters long.
        /// Empty sub-denoms are valid. The token will then be prefix + contract address, i.e. "factory/<bech32 address>/"
        sub_denom: String,
        amount: Uint128,
        recipient: String,
    },
    /// Swap over one or more pools
    /// Returns EstimatePriceResponse in the data field of the Response
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
}

impl From<OsmosisMsg> for CosmosMsg<OsmosisMsg> {
    fn from(msg: OsmosisMsg) -> CosmosMsg<OsmosisMsg> {
        CosmosMsg::Custom(msg)
    }
}

impl CustomMsg for OsmosisMsg {}
