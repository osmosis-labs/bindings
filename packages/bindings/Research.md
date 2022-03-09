# Research on Osmosis Entry Points

## GAMM Queries

### Provided Info

Four main areas:

* [PoolParams](https://github.com/osmosis-labs/osmosis/blob/v7.0.3/proto/osmosis/gamm/v1beta1/query.proto#L28-L35)
* [Assets and SpotPrice](https://github.com/osmosis-labs/osmosis/blob/v7.0.3/proto/osmosis/gamm/v1beta1/query.proto#L41-L48)
* [EstimateSwap](https://github.com/osmosis-labs/osmosis/blob/v7.0.3/proto/osmosis/gamm/v1beta1/query.proto#L50-L60)
* [Pools](https://github.com/osmosis-labs/osmosis/blob/main/proto/osmosis/gamm/v1beta1/query.proto#L17-L19)

SpotPrice requires knowing the PoolID:
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

As does Swap Estimation:
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

message QuerySwapExactAmountInResponse {
  string tokenOutAmount = 1 [
    (gogoproto.customtype) = "github.com/cosmos/cosmos-sdk/types.Int",
    (gogoproto.moretags) = "yaml:\"token_out_amount\"",
    (gogoproto.nullable) = false
  ];
}
```

Querying Pool or Pools, returns `google.protobuf.Any`, which is run to turn into Rust.
However, we can define an `enum` that captures all currently supported examples, currently only balancer pool.
Looking into the [details of balancer pool](https://github.com/osmosis-labs/osmosis/blob/main/proto/osmosis/gamm/pool-models/balancer/balancerPool.proto#L92-L133)
The interest is mainly in `uint64 id = 2;` (if listing) and `repeated osmosis.gamm.v1beta1.PoolAsset poolAssets = 6` to reflect what is in the pool.

[PoolAsset](https://github.com/osmosis-labs/osmosis/blob/main/proto/osmosis/gamm/v1beta1/pool.proto#L10-L30) contains a Coin
and a weight, and can be [directly queried from any pool id](https://github.com/osmosis-labs/osmosis/blob/main/proto/osmosis/gamm/v1beta1/query.proto#L108-L113)

### To expose

We may wish to eventually provide a way to paginate over all Pools and get their info, to implement some sort of Router.
However, for the original version, we can require the caller to know the pool id(s) that are of interest. Given
those ids, the contracts will want access to the following info:

Pool State:
* Current liquidity - list all coins in the pool (denom and amount)
* GAMM shares? - total number and denom

Estimate Immediate Trades:
* Spot Price (x A -> B on pool P)
* Estimate In (N A -> x B on pool P)
* Estimate Out (x A -> N B on pool P)

Price Oracle:
* TWAP (A -> B on Pool P)

### Future ideas

* List all pools
* Find pool given GAMM denom

## GAMM Messages

### Provided Info

* [Swapping](https://github.com/osmosis-labs/osmosis/blob/main/proto/osmosis/gamm/v1beta1/tx.proto#L12-L15) by either fixing input or output
* [Joining](https://github.com/osmosis-labs/osmosis/blob/main/proto/osmosis/gamm/v1beta1/tx.proto#L10) or [JoinSwap](https://github.com/osmosis-labs/osmosis/blob/main/proto/osmosis/gamm/v1beta1/tx.proto#L16-L19)
* [Exit](https://github.com/osmosis-labs/osmosis/blob/main/proto/osmosis/gamm/v1beta1/tx.proto#L11) or [ExitSwap](https://github.com/osmosis-labs/osmosis/blob/main/proto/osmosis/gamm/v1beta1/tx.proto#L20-L23)

While a contract staking LP shares is an interesing concept, the initial use will be limited to swapping for now.

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
This is out of scope for now, but would be an interesting future usecase.