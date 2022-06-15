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

- Make sure to create [2 accounts](https://github.com/osmosis-labs/cosmos-sdk/blob/83cb447d528595261b3220c658e5dc1f4b0df8fe/x/distribution/types/distribution.pb.go#L568) on the keyring - `test1` and `test2`

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
TX=$(osmosisd tx wasm store tokenfactory.wasm  --from test1 --chain-id=localosmosis --gas-prices 0.1uosmo --gas auto --gas-adjustment 1.3 -b block --output json -y | jq -r '.txhash')
CODE_ID=$(osmosisd query tx $TX --output json | jq -r '.logs[0].events[-1].attributes[0].value')
echo "Your contract code_id is $CODE_ID"
```

#### Instantiate the Contact
```sh
# Instantiate
osmosisd tx wasm instantiate $CODE_ID "{}" --amount 50000uosmo  --label "Token Factory Contract" --from test1 --chain-id localosmosis --gas-prices 0.1uosmo --gas auto --gas-adjustment 1.3 -b block -y --no-admin

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
osmosisd tx wasm execute $CONTRACT_ADDR '{ "create_denom": { "subdenom": "mydenom" } }' --from test1 --amount 10000000uosmo -b block

# If you do this
osmosisd q bank total --denom factory/$CONTRACT_ADDR/mydenom
# You should see this:
# amount: "0"
#denom: factory/osmo1wug8sewp6cedgkmrmvhl3lf3tulagm9hnvy8p0rppz9yjw0g4wtqcm3670/mydenom
```

- `Mint Tokens` executing from test1, minting to test2
```sh
TEST2_ADDR=osmo18s5lynnmx37hq4wlrw9gdn68sg2uxp5rgk26vv # This is from the result of "Download and Install LocalOsmosis" section

osmosisd tx wasm execute $CONTRACT_ADDR "{ \"mint_tokens\": {\"amount\": \"100\", \"denom\": \"factory/${CONTRACT_ADDR}/mydenom\", \"mint_to_address\": \"$TEST2_ADDR\"}}" --from test1 -b block

# If you do this
osmosisd q bank total --denom factory/$CONTRACT_ADDR/mydenom
# You should see this in the list:
# - amount: "100"
#   denom: factory/osmo14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9sq2r9
```

- `Burn Tokens` executing from test1, minting from test2

Currently, burning from an address other than "" which refers to `$CONTRACT_ADDR` is
not supported. If you attempt to burn from another address that
has a custom denom minted to but is not "" (empty string), you will get an error:

```sh
osmosisd tx wasm execute $CONTRACT_ADDR "{ \"burn_tokens\": {\"amount\": \"50\", \"denom\": \"factory/${CONTRACT_ADDR}/mydenom\", \"burn_from_address\": \"$CONTRACT_ADDR\"}}" --from test1 -b block

# You will see the following:
# raw_log: 'failed to execute message; message index: 0: address is not supported yet,
```

As a result, `Burn Tokens` be tested in the following ways:
- "pre-mint" the custom denom to the `$CONTRACT_ADDR` and then attempt to burn it from "" (empty string)
"burn_from_address"
- change admin to the address that has the custom denom minted to

Next, we will use the first "pre-mint" approach

```sh
# Pre-mint 100 of custom denom to $CONTRACT_ADDR
osmosisd tx wasm execute $CONTRACT_ADDR "{ \"mint_tokens\": {\"amount\": \"100\", \"denom\": \"factory/${CONTRACT_ADDR}/mydenom\", \"mint_to_address\": \"$CONTRACT_ADDR\"}}" --from test1 -b block

# Try to burn 50
osmosisd tx wasm execute $CONTRACT_ADDR "{ \"burn_tokens\": {\"amount\": \"50\", \"denom\": \"factory/${CONTRACT_ADDR}/mydenom\", \"burn_from_address\": \"\"}}" --from test1 -b block

# If you do this
osmosisd q bank total --denom factory/$CONTRACT_ADDR/mydenom
# You should see this in the list:
# - amount: "50"
#   denom: factory/osmo14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9sq2r9
```

- `Change Admin` executing from test1, changing from `$CONTRACT_ADDR` to $TEST2

```sh
TEST2_ADDR=osmo18s5lynnmx37hq4wlrw9gdn68sg2uxp5rgk26vv # This is from the result of "Download and Install LocalOsmosis" section

# Change Admin
osmosisd tx wasm execute $CONTRACT_ADDR "{ \"change_admin\": {\"denom\": \"factory/${CONTRACT_ADDR}/mydenom\", \"new_admin_address\": \"${TEST2_ADDR}\"}}" --from test1 -b block

# Verify New Admin
osmosisd q tokenfactory denom-authority-metadata factory/${CONTRACT_ADDR}/mydenom
# You should be able to see the following:
# osmosisd q tokenfactory denom-authority-metadata factory/${CONTRACT_ADDR}/mydenom
```

##### Queries

- `Get Denom`
```sh
osmosisd query wasm contract-state smart $CONTRACT_ADDR "{ \"get_denom\": {\"creator_address\": \"${CONTRACT_ADDR}\", \"subdenom\": \"mydenom\" }}"
```
