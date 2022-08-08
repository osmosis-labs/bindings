use cosmwasm_std::{QuerierWrapper, QueryRequest, StdResult};

use crate::query::{
    ArithmeticTwapResponse, ArithmeticTwapToNowResponse, FullDenomResponse, OsmosisQuery,
};

/// This is a helper wrapper to easily use our custom queries
pub struct OsmosisQuerier<'a> {
    querier: &'a QuerierWrapper<'a, OsmosisQuery>,
}

impl<'a> OsmosisQuerier<'a> {
    pub fn new(querier: &'a QuerierWrapper<OsmosisQuery>) -> Self {
        OsmosisQuerier { querier }
    }

    pub fn full_denom(
        &self,
        creator_addr: String,
        subdenom: String,
    ) -> StdResult<FullDenomResponse> {
        let full_denom_query = OsmosisQuery::FullDenom {
            creator_addr,
            subdenom,
        };
        let request: QueryRequest<OsmosisQuery> = OsmosisQuery::into(full_denom_query);
        self.querier.query(&request)
    }

    pub fn arithmetic_twap(
        &self,
        id: u64,
        quote_asset_denom: String,
        base_asset_denom: String,
        start_time: i64,
        end_time: i64,
    ) -> StdResult<ArithmeticTwapResponse> {
        let arithmetic_twap_query = OsmosisQuery::ArithmeticTwap {
            id,
            quote_asset_denom,
            base_asset_denom,
            start_time,
            end_time,
        };
        let request: QueryRequest<OsmosisQuery> = OsmosisQuery::into(arithmetic_twap_query);
        self.querier.query(&request)
    }

    pub fn arithmetic_twap_to_now(
        &self,
        id: u64,
        quote_asset_denom: String,
        base_asset_denom: String,
        start_time: i64,
    ) -> StdResult<ArithmeticTwapToNowResponse> {
        let arithmetic_twap_to_now_query = OsmosisQuery::ArithmeticTwapToNow {
            id,
            quote_asset_denom,
            base_asset_denom,
            start_time,
        };
        let request: QueryRequest<OsmosisQuery> = OsmosisQuery::into(arithmetic_twap_to_now_query);
        self.querier.query(&request)
    }
}
