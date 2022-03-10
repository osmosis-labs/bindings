use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::CustomQuery;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum OsmosisQuery {
    /// Given a subdenom minted by a contract via `OsmosisMsg::MintTokens`,
    /// returns the full denom as used by `BankMsg::Send`
    FullDenom { subdenom: String },
}

impl CustomQuery for OsmosisQuery {}

#[derive(Serialize, Deserialize, Default, Clone, PartialEq, JsonSchema, Debug)]
pub struct FullDenomResponse {
    pub denom: String,
}
