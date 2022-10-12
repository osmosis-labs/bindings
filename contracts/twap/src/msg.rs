use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Decimal;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GetArithmeticTwapResponse)]
    GetArithmeticTwap {
        id: u64,
        quote_asset_denom: String,
        base_asset_denom: String,
        start_time: i64,
        end_time: i64,
    },
    #[returns(GetArithmeticTwapToNowResponse)]
    GetArithmeticTwapToNow {
        id: u64,
        quote_asset_denom: String,
        base_asset_denom: String,
        start_time: i64,
    },
}

// We define a custom struct for each query response
#[cw_serde]
pub struct GetArithmeticTwapResponse {
    pub twap: Decimal,
}

#[cw_serde]
pub struct GetArithmeticTwapToNowResponse {
    pub twap: Decimal,
}
