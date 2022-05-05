use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::types::SwapAmountWithLimit;
use crate::{Step, Swap};
use cosmwasm_std::{CosmosMsg, CustomMsg, Uint128};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
/// A number of Custom messages that can call into the Osmosis bindings
pub enum OsmosisMsg {
    /// Swap over one or more pools
    /// Returns EstimatePriceResponse in the data field of the Response
    Swap {
        first: Swap,
        route: Vec<Step>,
        amount: SwapAmountWithLimit,
    },

    LockTokens {
        denom: String,
        amount: Uint128,
        duration: String
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
