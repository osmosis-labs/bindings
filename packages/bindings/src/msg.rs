use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CosmosMsg, CustomMsg, Uint128};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
/// A number of Custom messages that can call into the Osmosis bindings
pub enum OsmosisMsg {
    /// Contracts can mint native tokens that have an auto-generated denom
    /// namespaced under the contract's address. A contract may create any number
    /// of independent subdenoms.
    MintTokens {
        /// Must be 2-32 alphanumeric characters
        /// FIXME: revisit actual requirements in SDK
        subdenom: String,
        amount: Uint128,
        recipient: String,
    },
}

impl From<OsmosisMsg> for CosmosMsg<OsmosisMsg> {
    fn from(msg: OsmosisMsg) -> CosmosMsg<OsmosisMsg> {
        CosmosMsg::Custom(msg)
    }
}

impl CustomMsg for OsmosisMsg {}
