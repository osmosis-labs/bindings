# Osmosis Token Factory CosmWasm Contract Example

This demo contract provides a 1:1 mapping to the Osmosis token factory
bindings.

The contract messages only do some input validation and
directly call into their respective bindings outlined in the
"Messages" section below.

There are unit tests added to demonstrate how contract
developers might utilize `osmo-bindings-test` package
to import and use some test utilities.

## Messages

There are 4 messages:
- `ExecuteMsg::CreateDenom` maps to `OsmosisMsg::CreateDenom`
- `ExecuteMsg::ChangeAdmin` maps to `OsmosisMsg::ChangeAdmin`
- `ExecuteMsg::BurnTokens` maps to `OsmosisMsg::MintTokens`
- `ExecuteMsg::MintTokens` maps to `OsmosisMsg::BurnTokens`

## Query

1 query:
- `QueryMsg::GetDenom` maps to `OsmosisQuery::FullDenom`
