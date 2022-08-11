
# Osmosis Twap CosmWasm Contract Example

This demo contract provides a 1:1 mapping to the Osmosis twap
bindings.

The contract messages only do some input validation and
directly call into their respective bindings outlined in the
"Messages" section below.

The contract is only consisted of Queries outlined in the "Queries" section below, which serves the purpose of querying the twap state of the state machine. This example contract does not contain unique messages.

There are unit tests added to demonstrate how contract
developers might utilize `osmo-bindings-test` package
to import and use some test utilities.

## Query

There are 2 queries:
- `QueryMsg::GetArithmeticTwap` maps to `OsmosisQuery::ArithmeticTwap`
- `QueryMsg::GetArithmeticTwapToNow` maps to `OsmosisQuery::ArithmeticTwapToNow`

The time inputs within the queries are expected to be in Unix time nano second.

## Running with LocalOsmosis

### Download and Install LocalOsmosis

Please follow [this guide](https://docs.osmosis.zone/developing/dapps/get_started/cosmwasm-localosmosis.html#setup-localosmosis)
up and until and including ["Created a local key" section](https://docs.osmosis.zone/developing/dapps/get_started/cosmwasm-localosmosis.html#optimized-compilation)

- Make sure to create [2 accounts](https://github.com/osmosis-labs/cosmos-sdk/blob/83cb447d528595261b3220c658e5dc1f4b0df8fe/x/distribution/types/distribution.pb.go#L568) on the keyring - `test1` and `test2`

### Building

N.B.: All following example shell scripts assume executing them from the project root.

#### Compile Wasm

```sh
cd contracts/twap
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
TX=$(osmosisd tx wasm store twap.wasm  --from test1 --chain-id=localosmosis --gas-prices 0.1uosmo --gas auto --gas-adjustment 1.3 -b block --output json -y | jq -r '.txhash')
CODE_ID=$(osmosisd query tx $TX --output json | jq -r '.logs[0].events[-1].attributes[0].value')
echo "Your contract code_id is $CODE_ID"
```

#### Instantiate the Contact
```sh
# Instantiate
osmosisd tx wasm instantiate $CODE_ID "{}" --amount 50000uosmo  --label "Twap Contract" --from test1 --chain-id localosmosis --gas-prices 0.1uosmo --gas auto --gas-adjustment 1.3 -b block -y --no-admin

# Get contract address.
CONTRACT_ADDR=$(osmosisd query wasm list-contract-by-code $CODE_ID --output json | jq -r '.contracts[0]')
echo "Your contract address is $CONTRACT_ADDR"
```

#### Queries

You can generate the schema to assist you with determining the structure for each CLI query:

```sh
cd contracts/twap
cargo schema # generates schema in the contracts/twap/schema folder
```

For example, here is the schema for `GetArithmeticTwap` query:

```json
"get_arithmetic_twap": {
          "type": "object",
          "required": [
            "base_asset_denom",
            "end_time",
            "id",
            "quote_asset_denom",
            "start_time"
          ],
          "properties": {
            "base_asset_denom": {
              "type": "string"
            },
            "end_time": {
              "type": "integer",
              "format": "int64"
            },
            "id": {
              "type": "integer",
              "format": "uint64",
              "minimum": 0.0
            },
            "quote_asset_denom": {
              "type": "string"
            },
            "start_time": {
              "type": "integer",
              "format": "int64"
            }
          }
        }
```

##### Queries

- `Get Arithmetic Twap`
```sh
osmosisd query wasm contract-state smart $CONTRACT_ADDR "{ \"get_arithmetic_twap\": {\"id\": 1 \"$\", \"base_asset_denom\": \"denom1\", \"quote_asset_denom\": \"denom2\", \"start_time\": 10 , \"end_time\": 20 }}"
```

- `Get Arithmetic Twap To Now`
```sh
osmosisd query wasm contract-state smart $CONTRACT_ADDR "{ \"get_arithmetic_twap\": {\"id\": 1 \"$\", \"base_asset_denom\": \"denom1\", \"quote_asset_denom\": \"denom2\", \"start_time\": 10}}"
```