use cosmwasm_std::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetArithmeticTwap {
        id: u64,
        quote_asset_denom: String,
        base_asset_denom: String,
        start_time: i64,
        end_time: i64,
    },
    GetArithmeticTwapToNow {
        id: u64,
        quote_asset_denom: String,
        base_asset_denom: String,
        start_time: i64,
    },
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct GetArithmeticTwapResponse {
    pub twap: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct GetArithmeticTwapToNowResponse {
    pub twap: Decimal,
}
