use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::types::{Step, Swap, SwapAmount};
use cosmwasm_std::{Coin, CustomQuery, Decimal, Uint128};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum OsmosisQuery {
    /// Given a subdenom created by the address `creator_addr` via `OsmosisMsg::CreateDenom`,
    /// returns the full denom as used by `BankMsg::Send`.
    /// You may call `FullDenom { creator_addr: env.contract.address, subdenom }` to find the denom issued
    /// by the current contract.
    FullDenom {
        creator_addr: String,
        subdenom: String,
    },
    /// For a given pool ID, list all tokens traded on it with current liquidity (spot).
    /// As well as the total number of LP shares and their denom
    PoolState { id: u64 },
    /// Return current spot price swapping In for Out on given pool ID.
    /// Warning: this can easily be manipulated via sandwich attacks, do not use as price oracle.
    /// We will add TWAP for more robust price feed.
    SpotPrice { swap: Swap, with_swap_fee: bool },
    /// Return current spot price swapping In for Out on given pool ID.
    /// You can call `EstimateSwap { contract: env.contract.address, ... }` to set sender to the
    /// current contract.
    /// Warning: this can easily be manipulated via sandwich attacks, do not use as price oracle.
    /// We will add TWAP for more robust price feed.
    EstimateSwap {
        sender: String,
        first: Swap,
        route: Vec<Step>,
        amount: SwapAmount,
    },
}

impl CustomQuery for OsmosisQuery {}

impl OsmosisQuery {
    /// Calculate spot price without swap fee
    pub fn spot_price(pool_id: u64, denom_in: &str, denom_out: &str) -> Self {
        OsmosisQuery::SpotPrice {
            swap: Swap::new(pool_id, denom_in, denom_out),
            with_swap_fee: false,
        }
    }

    /// Basic helper to estimate price of a swap on one pool
    pub fn estimate_swap(
        contract: impl Into<String>,
        pool_id: u64,
        denom_in: impl Into<String>,
        denom_out: impl Into<String>,
        amount: SwapAmount,
    ) -> Self {
        OsmosisQuery::EstimateSwap {
            sender: contract.into(),
            first: Swap::new(pool_id, denom_in, denom_out),
            amount,
            route: vec![],
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct FullDenomResponse {
    pub denom: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct PoolStateResponse {
    /// The various assets that be swapped. Including current liquidity.
    pub assets: Vec<Coin>,
    /// The number of lp shares and their amount
    pub shares: Coin,
}

impl PoolStateResponse {
    pub fn has_denom(&self, denom: &str) -> bool {
        self.assets.iter().any(|c| c.denom == denom)
    }

    pub fn lp_denom(&self) -> &str {
        &self.shares.denom
    }

    /// If I hold num_shares of the lp_denom, how many assets does that equate to?
    pub fn shares_value(&self, num_shares: impl Into<Uint128>) -> Vec<Coin> {
        let num_shares = num_shares.into();
        self.assets
            .iter()
            .map(|c| Coin {
                denom: c.denom.clone(),
                amount: c.amount * num_shares / self.shares.amount,
            })
            .collect()
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct SpotPriceResponse {
    /// How many output we would get for 1 input
    pub price: Decimal,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct SwapResponse {
    // If you query with SwapAmount::Input, this is SwapAmount::Output
    // If you query with SwapAmount::Output, this is SwapAmount::Input
    pub amount: SwapAmount,
}
