use std::marker::PhantomData;

use osmo_bindings::OsmosisQuery;

use crate::errors::ReflectError;
use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{Binary, Coin, ContractResult, OwnedDeps, SystemResult};

/// A drop-in replacement for cosmwasm_std::testing::mock_dependencies
/// this uses our CustomQuerier.
pub fn mock_dependencies_with_custom_querier(
    contract_balance: &[Coin],
) -> OwnedDeps<MockStorage, MockApi, MockQuerier<OsmosisQuery>, OsmosisQuery> {
    let custom_querier: MockQuerier<OsmosisQuery> =
        MockQuerier::new(&[(MOCK_CONTRACT_ADDR, contract_balance)])
            .with_custom_handler(|query| SystemResult::Ok(custom_query_execute(query)));
    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: custom_querier,
        custom_query_type: PhantomData,
    }
}

pub fn custom_query_execute(_query: &OsmosisQuery) -> ContractResult<Binary> {
    let err = ReflectError::NotYetImplemented;
    ContractResult::Err(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{from_binary, QuerierWrapper, QueryRequest};

    #[test]
    fn custom_query_execute_ping() {
        let res = custom_query_execute(&OsmosisQuery::Ping {}).unwrap();
        let response: SpecialResponse = from_binary(&res).unwrap();
        assert_eq!(response.msg, "pong");
    }

    #[test]
    fn custom_query_execute_capitalize() {
        let res = custom_query_execute(&OsmosisQuery::Capitalized {
            text: "fOObaR".to_string(),
        })
        .unwrap();
        let response: SpecialResponse = from_binary(&res).unwrap();
        assert_eq!(response.msg, "FOOBAR");
    }

    #[test]
    fn custom_querier() {
        let deps = mock_dependencies_with_custom_querier(&[]);
        let req: QueryRequest<_> = OsmosisQuery::Capitalized {
            text: "food".to_string(),
        }
        .into();
        let wrapper = QuerierWrapper::new(&deps.querier);
        let response: SpecialResponse = wrapper.query(&req).unwrap();
        assert_eq!(response.msg, "FOOD");
    }
}
