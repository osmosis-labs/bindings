# Osmosis Bindings

CosmWasm bindings to custom Osmosis features.

## Prerequisites

Before starting, make sure you have [rustup](https://rustup.rs/) along with a
recent `rustc` and `cargo` version installed. Currently, we are testing on 1.58.1+.

And you need to have the `wasm32-unknown-unknown` target installed as well.

You can check that via:

```sh
rustc --version
cargo --version
rustup target list --installed
# if wasm32 is not listed above, run this
rustup target add wasm32-unknown-unknown
```

## running tests
```
cargo test --locked
```

## Compile
```
cargo build --locked
```

## generate schema for osmosis contract

```
cd contracts/reflect
cargo schema --locked
```
This will give you a schema at contracts/reflect/schema/


## build contract

```
cd contracts/reflect
cargo wasm
```


### Understanding the tests

The main code is in `src/contract.rs` and the unit tests there run in pure rust,
which makes them very quick to execute and give nice output on failures, especially
if you do `RUST_BACKTRACE=1 cargo unit-test`.

We consider testing critical for anything on a blockchain, and recommend to always keep
the tests up to date.

## Generating JSON Schema

While the Wasm calls (`instantiate`, `execute`, `query`) accept JSON, this is not enough
information to use it. We need to expose the schema for the expected messages to the
clients. You can generate this schema by calling `cargo schema`, which will output
4 files in `./schema`, corresponding to the 3 message types the contract accepts,
as well as the internal `State`.

These files are in standard json-schema format, which should be usable by various
client side tools, either to auto-generate codecs, or just to validate incoming
json wrt. the defined schema.

## Preparing the Wasm bytecode for production

Before we upload it to a chain, we need to ensure the smallest output size possible,
as this will be included in the body of a transaction. We also want to have a
reproducible build process, so third parties can verify that the uploaded Wasm
code did indeed come from the claimed rust code.

To solve both these issues, we have produced `rust-optimizer`, a docker image to
produce an extremely small build output in a consistent manner. The suggest way
to run it is this:

```sh
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/workspace-optimizer:0.12.6
```

Or, If you're on an arm64 machine, you should use a docker image built with arm64.
```sh
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/workspace-optimizer-arm64:0.12.6
```

We must mount the contract code to `/code`. You can use a absolute path instead
of `$(pwd)` if you don't want to `cd` to the directory first. The other two
volumes are nice for speedup. Mounting `/code/target` in particular is useful
to avoid docker overwriting your local dev files with root permissions.
Note the `/code/target` cache is unique for each contract being compiled to limit
interference, while the registry cache is global.

This is rather slow compared to local compilations, especially the first compile
of a given contract. The use of the two volume caches is very useful to speed up
following compiles of the same contract.

This produces an `artifacts` directory with a `PROJECT_NAME.wasm`, as well as
`checksums.txt`, containing the Sha256 hash of the wasm file.
The wasm file is compiled deterministically (anyone else running the same
docker on the same git commit should get the identical file with the same Sha256 hash).
It is also stripped and minimized for upload to a blockchain (we will also
gzip it in the uploading process to make it even smaller).


## Writing contracts that interact with Osmosis

### Choosing a network

To use these bindings in your contracts, you'll need to either deploy them to
the testnet or have a localnet setup. Read [the official
docs](https://docs.osmosis.zone/developing/dapps/get_started/) to learn how to
do this.

Note that Cosmwasm on Osmosis is permissioned, so you'll have to disabled
permissioned Cosmwasm for easier development.

## Parametrizing your inputs and results with the Osmosis types

in order for the integration to work the following types, defined in this
bindings, will need to be used as the type of your inputs/outputs or to
parametrize Cosmwasm types:

 * `OsmosisQuery`, which implements `CustomQuery`
 * `OsmosisMsg`, which implement `CosmosMsg`

 Specifically, any function that uses `Deps` or `DepsMut` and needs to interact
 with the chain will need to parametrize them as `Deps<OsmosisQuery>` and
 `DepsMut<OsmosisQuery>`, and any `Response` that adds messages or submessages
 to be executed needs to be parametrized as `Response<OsmosisMsg>`.

## Executing Osmosis queries

If you want to execute Osmosis queries inside your contract, you can do this
without the need of message passing (see the Cosmwasm documentation on
[QuerySemantics](https://github.com/CosmWasm/cosmwasm/blob/main/SEMANTICS.md#query-semantics)).

To do this you can create an `OsmosisQuery` and a `QueryRequest<OsmosisQuery>`
to be dispatched by the querier.

The folowing code createds a `PoolState` variant of `OsmosisQuery` and passes it
to the querier:

``` rust
    let pool_query = OsmosisQuery::PoolState { id: 1 };
    let query = QueryRequest::from(pool_query);
    let pool_info: PoolStateResponse = deps.querier.query(&query)?;
```

The following queries the spot price of two denoms but uses the helper
`spot_price` function of `OsmosisQuery` to simplify the query creation:

``` rust
    let spot_price = OsmosisQuery::spot_price(1, &denom1, &denom2);
    let query = QueryRequest::from(spot_price);
    let response: SpotPriceResponse = deps.querier.query(&query)?;
```

Please note that the `deps` used in both these queries need to be of type
`Deps<OsmosisQuery>` (or `DepsMut<OsmosisQuery>`). Otherwise the default
implementation would assume an `Empty` custom query,

## Executting transactions as (sub)messages

To execute osmosis transactions as part of your contract's execute response. You
can create create the `OsmosisMsg` and provide it as part of your contract's response.

Here is an example of how to execute a swap as part of the

See the Cosmwasm documentation on
[submessages](https://github.com/CosmWasm/cosmwasm/blob/main/SEMANTICS.md#dispatching-messages)
for more details about how this gets executed and how to handle replies.

``` rust
fn execute_swap(
    _deps: DepsMut,
    _info: MessageInfo,
    input: u128,
    min_output: u128,
) -> Result<Response<OsmosisMsg>, ContractError> {
    let swap = OsmosisMsg::simple_swap(
        1,
        "uosmo",
        "uion",
        SwapAmountWithLimit::ExactIn {
            input: Uint128::from(input),
            min_output: Uint128::from(min_output),
        },
    );
    let msgs = vec![SubMsg::new(swap)];

    Ok(Response::new()
        .add_attribute("action", "execute_swap")
        .add_submessages(msgs))
}

```

Note that `DepsMut` is not parametrized in that function as it's not being used
in it (so the default `DepsMut<Empty>` sufices). However, if your contract is
using the `deps` somewhere else, chances are you will want to unify the types
along your contract, so having the input parameter be `DepsMut<OsmosisQuery>` is
probably a good idea.

For more information on the parameters of the `OsmosisMsg` used above, see the
Osmosis Cosmwasm API documentation (TBD).

# Executing custom transactions

If the transaction you want to execute is not provided by this API, you can
still execute it using `CustomMsg`.

ToDo: Add an example.

## Writing integration tests that mock the Osmosis responses


To be able to write tests for the functions that depend on interacting with
Osmosis, you will need to mock the chain so that it can handle your contract
requests and provide the right responses.

To do that, you will need to:

### Create and initialize an app

``` rust
        let mut app = OsmosisApp::new();
        //... setup your variables here
        app.init_modules(|router, _, storage| {
            router.custom.set_pool(storage, 1, &pool).unwrap();
            router
                .bank
                .init_balance(storage, &owner, init_funds)
                .unwrap();
            router
                .bank
                .init_balance(storage, &borrower, pool_funds.clone())
                .unwrap();
        });

```

### Define a wrapper for your contract

``` rust
    pub fn contract<C, Q>() -> Box<dyn Contract<C, Q>>
    where
        C: Clone + Debug + PartialEq + JsonSchema + DeserializeOwned + 'static,
        Q: CustomQuery + DeserializeOwned + 'static,
        ContractWrapper<
            ExecuteMsg,
            InstantiateMsg,
            QueryMsg,
            ContractError,
            ContractError,
            cosmwasm_std::StdError,
            OsmosisMsg,
            OsmosisQuery,
        >: Contract<C, Q>,
    {
        let contract = ContractWrapper::new(execute, instantiate, query); //.with_reply(reply);
        Box::new(contract)
    }
```


### Instantiate your contract


``` rust

        let contract: Box<dyn Contract<OsmosisMsg, OsmosisQuery>> = contract();
        let code_id = app.store_code(contract);

        // Instantiate the contract
        let msg = InstantiateMsg {
            admin: None,
            funds_denom: "usdc".to_string(),
            collateral_denom: "gamm/pool/1".to_string(),
        };
        let contract_addr = app
            .instantiate_contract(code_id, owner.clone(), &msg, &[], "shark", None)
            .unwrap();
```


### Execute your messages and test the results

``` rust
        let balance = app.wrap().query_balance(&borrower, "usdc").unwrap();
        assert_eq!(balance.amount, Uint128::new(0));

        let amount = 6;
        let msg = ExecuteMsg::Borrow { amount };  // This is a message defined by your contract

        let wasm_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: contract_addr.into(),
            msg: to_binary(&&inner_msg).unwrap(),
            funds: vec![],
        });

        app.execute(borrower.clone(), wasm_msg).unwrap();

        let balance = app.wrap().query_balance(&borrower, "usdc").unwrap();
        assert_eq!(balance.amount, Uint128::new(amount));
```
