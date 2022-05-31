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
- `ExecuteMsg::BurnTokens` maps to `OsmosisMsg::BurnTokens`
- `ExecuteMsg::MintTokens` maps to `OsmosisMsg::MintTokens`

## Query

1 query:
- `QueryMsg::GetDenom` maps to `OsmosisQuery::FullDenom`

## Running with LocalOsmosis

### Download and Install LocalOsmosis

Please follow [this guide](https://docs.osmosis.zone/developing/dapps/get_started/cosmwasm-localosmosis.html#setup-localosmosis)
up and until and including ["Created a local key" section](https://docs.osmosis.zone/developing/dapps/get_started/cosmwasm-localosmosis.html#optimized-compilation)

### Building

N.B.: All following example shell scripts assume executing them from the project root.

#### Compile Wasm

```sh
cd contracts/tokenfactory
rustup default stable
cargo wasm
```

#### Optimize Compilation

```sh
sudo docker run --rm -v "$(pwd)":/code   --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target   --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry   cosmwasm/workspace-optimizer:0.12.6
```

#### Upload Contract to LocalOsmosis

```sh
cd artifacts

# Upload and store transaction hash in TX environment variable.
TX=$(osmosisd tx wasm store tokenfactory.wasm  --from <unsafe-test-key-name> --chain-id=localosmosis --gas-prices 0.1uosmo --gas auto --gas-adjustment 1.3 -b block --output json -y | jq -r '.txhash')
CODE_ID=$(osmosisd query tx $TX --output json | jq -r '.logs[0].events[-1].attributes[0].value')
echo "Your contract code_id is $CODE_ID"
```

#### Instantiate the Contact
```sh
# Instantiate
osmosisd tx wasm instantiate $CODE_ID "{}" --amount 50000uosmo  --label "Token Factory Contract" --from <unsafe-test-key-name> --chain-id localosmosis --gas-prices 0.1uosmo --gas auto --gas-adjustment 1.3 -b block -y --no-admin

# Get contract address.
CONTRACT_ADDR=$(osmosisd query wasm list-contract-by-code $CODE_ID --output json | jq -r '.contracts[0]')
echo "Your contract address is $CONTRACT_ADDR"
```

#### Execute & Queries

You can generate the schema to assist you with determining the structure for each CLI query:

```sh
cd contracts/tokenfactory
carge schema # generates schema in the contracts/tokenfactory/schema folder
```

For example, here is the schema for `CreateDenom` message:

```json
{
      "type": "object",
      "required": [
        "create_denom"
      ],
      "properties": {
        "create_denom": {
          "type": "object",
          "required": [
            "subdenom"
          ],
          "properties": {
            "subdenom": {
              "type": "string"
            }
          }
        }
      },
      "additionalProperties": false
    }
```

##### Messages

- `Create Denom`
```sh
osmosisd tx wasm execute $CONTRACT_ADDR '{ "subdenom": "mydenom" }' --from test1
```

Other messages can be executed similarly.

##### Queries

- `Get Denom`
```sh
osmosisd query wasm contract-state smart $CONTRACT_ADDR "{ \"get_denom\": {\"creator_address\": \"${CONTRACT_ADDR}\", \"subdenom\": \"mydenom\" }}"
```
