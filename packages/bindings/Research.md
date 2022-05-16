# Research on Osmosis Entry Points

## GAMM Queries

### Pool Info

* [PoolParams](https://github.com/osmosis-labs/osmosis/blob/v7.0.3/proto/osmosis/gamm/v1beta1/query.proto#L28-L35)
* [Pools](https://github.com/osmosis-labs/osmosis/blob/main/proto/osmosis/gamm/v1beta1/query.proto#L17-L19)

Querying Pool or Pools, returns `google.protobuf.Any`, which is hard to turn into Rust.
However, we can define an `enum` that captures all currently supported examples; currently only balancer pool.
Looking into the [details of balancer pool](https://github.com/osmosis-labs/osmosis/blob/main/proto/osmosis/gamm/pool-models/balancer/balancerPool.proto#L92-L133)
The interest is mainly in `uint64 id = 2` (if listing), and `repeated osmosis.gamm.v1beta1.PoolAsset poolAssets = 6`, to reflect what is in the pool.

There are 1000s of pools, many unused. There is no reasonable way to list over all Pools in contracts.
What we can currently do is extract the [Pool ID from the LP token denom](https://github.com/osmosis-labs/osmosis/blob/e13cddc698a121dce2f8919b2a0f6a743f4082d6/x/gamm/types/key.go#L52-L54).
Osmosis may add a lookup from a denom to a list of all incentivized pools where that denom is traded, which could be a first step to routing.

### Pool State

[PoolAsset](https://github.com/osmosis-labs/osmosis/blob/main/proto/osmosis/gamm/v1beta1/pool.proto#L10-L30) contains a Coin
and a weight, and can be [directly queried from any pool id](https://github.com/osmosis-labs/osmosis/blob/main/proto/osmosis/gamm/v1beta1/query.proto#L108-L113)
This will let us know both what tokens can be traded on the pool, as well as the current liquidity.

We can also [query total shares](https://github.com/osmosis-labs/osmosis/blob/main/proto/osmosis/gamm/v1beta1/query.proto#L97-L105),
which combined with the pool assets, will let us know how many tokens are backed by each GAMM token.

```proto
message QueryTotalSharesRequest {
  uint64 poolId = 1 [ (gogoproto.moretags) = "yaml:\"pool_id\"" ];
}
message QueryTotalSharesResponse {
  cosmos.base.v1beta1.Coin totalShares = 1 [
    (gogoproto.moretags) = "yaml:\"total_shares\"",
    (gogoproto.nullable) = false
  ];
}

message QueryPoolAssetsRequest {
  uint64 poolId = 1 [ (gogoproto.moretags) = "yaml:\"pool_id\"" ];
}
message QueryPoolAssetsResponse {
  repeated PoolAsset poolAssets = 1 [ (gogoproto.nullable) = false ];
}

message PoolAsset {
  // Coins we are talking about,
  // the denomination must be unique amongst all PoolAssets for this pool.
  cosmos.base.v1beta1.Coin token = 1
      [ (gogoproto.moretags) = "yaml:\"token\"", (gogoproto.nullable) = false ];
  // Weight that is not normalized. This weight must be less than 2^50
  string weight = 2 [
    (gogoproto.customtype) = "github.com/cosmos/cosmos-sdk/types.Int",
    (gogoproto.moretags) = "yaml:\"weight\"",
    (gogoproto.nullable) = false
  ];
}
```

### Price Checks

* [SpotPrice](https://github.com/osmosis-labs/osmosis/blob/v7.0.3/proto/osmosis/gamm/v1beta1/query.proto#L45-L48)
* [EstimateSwap](https://github.com/osmosis-labs/osmosis/blob/v7.0.3/proto/osmosis/gamm/v1beta1/query.proto#L50-L60)

SpotPrice requires knowing the PoolID and in/out tokens.
It only allows a check for a swap on one pool, not a series:

```proto
message QuerySpotPriceRequest {
    uint64 poolId = 1 [ (gogoproto.moretags) = "yaml:\"pool_id\"" ];
    string tokenInDenom = 2 [ (gogoproto.moretags) = "yaml:\"token_in_denom\"" ];
    string tokenOutDenom = 3 [ (gogoproto.moretags) = "yaml:\"token_out_denom\"" ];
    bool withSwapFee = 4 [ (gogoproto.moretags) = "yaml:\"with_swap_fee\"" ];
}
message QuerySpotPriceResponse {
    // String of the Dec. Ex) 10.203uatom
    string spotPrice = 1 [ (gogoproto.moretags) = "yaml:\"spot_price\"" ];
}
```

Swap Estimation takes a list of pools to be used in the swap.
You set either the input or output as fixed, and it will estimate the return on the other side.
You can either say "how many OSMO do I get for exactly 10 ATOM?" or
"how many ATOM must I swap to get exactly 30 OSMO?".

```proto
message QuerySwapExactAmountInRequest {
  string sender = 1 [ (gogoproto.moretags) = "yaml:\"sender\"" ];
  uint64 poolId = 2 [ (gogoproto.moretags) = "yaml:\"pool_id\"" ];
  string tokenIn = 3 [ (gogoproto.moretags) = "yaml:\"token_in\"" ];
  repeated SwapAmountInRoute routes = 4 [
    (gogoproto.moretags) = "yaml:\"routes\"",
    (gogoproto.nullable) = false
  ];
}

message SwapAmountInRoute {
  uint64 poolId = 1 [ (gogoproto.moretags) = "yaml:\"pool_id\"" ];
  string tokenOutDenom = 2
      [ (gogoproto.moretags) = "yaml:\"token_out_denom\"" ];
}

message QuerySwapExactAmountInResponse {
  string tokenOutAmount = 1 [
    (gogoproto.customtype) = "github.com/cosmos/cosmos-sdk/types.Int",
    (gogoproto.moretags) = "yaml:\"token_out_amount\"",
    (gogoproto.nullable) = false
  ];
}
```

An important question is how to best represent these routes in Rust types.

It is effectively one of these...

* `A -(P1)-> B`
* `A -(P1)-> B -(P2)-> C`
* `A -(P1)-> B -(P2)-> C -(P3)-> D`

Should that be:
* Input + N (pool + output) Where N >= 1 is enforced runtime?
* Define first swap and then an optional list of chains? (type safety that we cannot add N == 0)

**Note**: The first element is currently being enforced at the structs level. See [EstimateSwap](./src/query.rs)
and [Swap](./src/msg.rs).

### To be defined

Price Oracle needs **TWAP** `(A -> B on Pool P)` over last hour/day/etc.
Some simple version will be included in Osmosis 8.0.

Integration to be defined

## GAMM Messages

### Provided Info

* [Swapping](https://github.com/osmosis-labs/osmosis/blob/main/proto/osmosis/gamm/v1beta1/tx.proto#L12-L15) by either fixing input or output
* [Joining](https://github.com/osmosis-labs/osmosis/blob/main/proto/osmosis/gamm/v1beta1/tx.proto#L10) or [JoinSwap](https://github.com/osmosis-labs/osmosis/blob/main/proto/osmosis/gamm/v1beta1/tx.proto#L16-L19)
* [Exit](https://github.com/osmosis-labs/osmosis/blob/main/proto/osmosis/gamm/v1beta1/tx.proto#L11) or [ExitSwap](https://github.com/osmosis-labs/osmosis/blob/main/proto/osmosis/gamm/v1beta1/tx.proto#L20-L23)

While a contract staking LP shares is an interesting concept, the initial use will be limited to swapping for now.

```proto
message SwapAmountInRoute {
  uint64 poolId = 1 [ (gogoproto.moretags) = "yaml:\"pool_id\"" ];
  string tokenOutDenom = 2
      [ (gogoproto.moretags) = "yaml:\"token_out_denom\"" ];
}

message MsgSwapExactAmountIn {
  string sender = 1 [ (gogoproto.moretags) = "yaml:\"sender\"" ];
  repeated SwapAmountInRoute routes = 2 [ (gogoproto.nullable) = false ];
  cosmos.base.v1beta1.Coin tokenIn = 3 [
    (gogoproto.moretags) = "yaml:\"token_in\"",
    (gogoproto.nullable) = false
  ];
  string tokenOutMinAmount = 4 [
    (gogoproto.customtype) = "github.com/cosmos/cosmos-sdk/types.Int",
    (gogoproto.moretags) = "yaml:\"token_out_min_amount\"",
    (gogoproto.nullable) = false
  ];
}
```

```proto
message SwapAmountOutRoute {
  uint64 poolId = 1 [ (gogoproto.moretags) = "yaml:\"pool_id\"" ];
  string tokenInDenom = 2 [ (gogoproto.moretags) = "yaml:\"token_out_denom\"" ];
}

message MsgSwapExactAmountOut {
  string sender = 1 [ (gogoproto.moretags) = "yaml:\"sender\"" ];
  repeated SwapAmountOutRoute routes = 2 [ (gogoproto.nullable) = false ];
  string tokenInMaxAmount = 3 [
    (gogoproto.customtype) = "github.com/cosmos/cosmos-sdk/types.Int",
    (gogoproto.moretags) = "yaml:\"token_in_max_amount\"",
    (gogoproto.nullable) = false
  ];
  cosmos.base.v1beta1.Coin tokenOut = 4 [
    (gogoproto.moretags) = "yaml:\"token_out\"",
    (gogoproto.nullable) = false
  ];
}
```

### To expose

What you basically need to do is establish a chain of token swaps:

`A -(P1)-> B -(P2)-> C`

You then set either:

* fixed amount A in, min amount C out
* max amount A in, fixed amount C out

Question: would min/max amount or min/max price be clearer to the user API?

## Locking / Staking LP shares

See [lockup tx](https://github.com/osmosis-labs/osmosis/blob/main/proto/osmosis/lockup/tx.proto) and [lockup queries](https://github.com/osmosis-labs/osmosis/blob/main/proto/osmosis/lockup/query.proto).
This is out of scope for now, but would be an interesting future use case.
