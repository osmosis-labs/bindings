use cosmwasm_std::{QuerierWrapper, QueryRequest, StdResult};

use crate::query::{FullDenomResponse, OsmosisQuery};

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
}
