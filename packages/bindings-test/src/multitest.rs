use anyhow::{bail, Result as AnyResult};
use itertools::Itertools;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::cmp::max;
use std::fmt::Debug;
use std::iter;
use std::ops::{Deref, DerefMut};
use thiserror::Error;

use cosmwasm_std::testing::{MockApi, MockStorage};
use cosmwasm_std::{
    coins, to_binary, Addr, Api, BankMsg, Binary, BlockInfo, Coin, CustomQuery, Decimal, Empty,
    Fraction, Isqrt, Querier, QuerierResult, StdError, StdResult, Storage, Uint128,
};
use cw_multi_test::{
    App, AppResponse, BankKeeper, BankSudo, BasicAppBuilder, CosmosRouter, Module, WasmKeeper,
};
use cw_storage_plus::Map;

use crate::error::ContractError;
use osmo_bindings::{
    FullDenomResponse, OsmosisMsg, OsmosisQuery, PoolStateResponse, SpotPriceResponse, Step, Swap,
    SwapAmount, SwapAmountWithLimit, SwapResponse,
};

pub const POOLS: Map<u64, Pool> = Map::new("pools");

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Pool {
    pub assets: Vec<Coin>,
    pub shares: Uint128,
    pub fee: Decimal,
}

impl Pool {
    // make an equal-weighted uniswap-like pool with 0.3% fees
    pub fn new(a: Coin, b: Coin) -> Self {
        let shares = (a.amount * b.amount).isqrt();
        Pool {
            assets: vec![a, b],
            shares,
            fee: Decimal::permille(3),
        }
    }

    pub fn has_denom(&self, denom: &str) -> bool {
        self.assets.iter().any(|c| c.denom == denom)
    }

    pub fn get_amount(&self, denom: &str) -> Option<Uint128> {
        self.assets
            .iter()
            .find(|c| c.denom == denom)
            .map(|c| c.amount)
    }

    pub fn set_amount(&mut self, denom: &str, amount: Uint128) -> Result<(), OsmosisError> {
        let pos = self
            .assets
            .iter()
            .position(|c| c.denom == denom)
            .ok_or(OsmosisError::AssetNotInPool)?;
        self.assets[pos].amount = amount;
        Ok(())
    }

    pub fn spot_price(
        &self,
        denom_in: &str,
        denom_out: &str,
        with_swap_fee: bool,
    ) -> Result<Decimal, OsmosisError> {
        // ensure they have both assets
        let (bal_in, bal_out) = match (self.get_amount(denom_in), self.get_amount(denom_out)) {
            (Some(a), Some(b)) => (a, b),
            _ => return Err(OsmosisError::AssetNotInPool),
        };
        let mult = if with_swap_fee {
            Decimal::one() - self.fee
        } else {
            Decimal::one()
        };
        let price = Decimal::from_ratio(bal_out * mult, bal_in);
        Ok(price)
    }

    pub fn swap(
        &mut self,
        denom_in: &str,
        denom_out: &str,
        amount: SwapAmount,
    ) -> Result<SwapAmount, OsmosisError> {
        // ensure they have both assets
        let (bal_in, bal_out) = match (self.get_amount(denom_in), self.get_amount(denom_out)) {
            (Some(a), Some(b)) => (a, b),
            _ => return Err(OsmosisError::AssetNotInPool),
        };
        // do calculations (in * out = k) equation
        let (final_in, final_out, payout) = match amount {
            SwapAmount::In(input) => {
                let input_minus_fee = input * (Decimal::one() - self.fee);
                let final_out = bal_in * bal_out / (bal_in + input_minus_fee);
                let payout = SwapAmount::Out(bal_out - final_out);
                let final_in = bal_in + input;
                (final_in, final_out, payout)
            }
            SwapAmount::Out(output) => {
                let in_without_fee = bal_in * bal_out / (bal_out - output);
                // add one to handle rounding (final_in - old_in) / (1 - fee)
                let mult = Decimal::one() - self.fee;
                // Use this as Uint128 / Decimal is not implemented in cosmwasm_std
                let pay_incl_fee = (in_without_fee - bal_in) * mult.denominator()
                    / mult.numerator()
                    + Uint128::new(1);

                let payin = SwapAmount::In(pay_incl_fee);
                let final_in = bal_in + pay_incl_fee;
                let final_out = bal_out - output;
                (final_in, final_out, payin)
            }
        };
        // update internal balance
        self.set_amount(denom_in, final_in)?;
        self.set_amount(denom_out, final_out)?;
        Ok(payout)
    }

    pub fn swap_with_limit(
        &mut self,
        denom_in: &str,
        denom_out: &str,
        amount: SwapAmountWithLimit,
    ) -> Result<SwapAmount, OsmosisError> {
        match amount {
            SwapAmountWithLimit::ExactIn { input, min_output } => {
                let payout = self.swap(denom_in, denom_out, SwapAmount::In(input))?;
                if payout.as_out() < min_output {
                    Err(OsmosisError::PriceTooLow)
                } else {
                    Ok(payout)
                }
            }
            SwapAmountWithLimit::ExactOut { output, max_input } => {
                let payin = self.swap(denom_in, denom_out, SwapAmount::Out(output))?;
                if payin.as_in() > max_input {
                    Err(OsmosisError::PriceTooLow)
                } else {
                    Ok(payin)
                }
            }
        }
    }

    pub fn gamm_denom(&self, pool_id: u64) -> String {
        // see https://github.com/osmosis-labs/osmosis/blob/e13cddc698a121dce2f8919b2a0f6a743f4082d6/x/gamm/types/key.go#L52-L54
        format!("gamm/pool/{}", pool_id)
    }

    pub fn into_response(self, pool_id: u64) -> PoolStateResponse {
        let denom = self.gamm_denom(pool_id);
        PoolStateResponse {
            assets: self.assets,
            shares: Coin {
                denom,
                amount: self.shares,
            },
        }
    }
}

pub struct OsmosisModule {}

/// How many seconds per block
/// (when we increment block.height, use this multiplier for block.time)
pub const BLOCK_TIME: u64 = 5;

impl OsmosisModule {
    fn build_denom(&self, creator: &Addr, subdenom: &str) -> Result<String, ContractError> {
        // Minimum validation checks on the full denom.
        // https://github.com/cosmos/cosmos-sdk/blob/2646b474c7beb0c93d4fafd395ef345f41afc251/types/coin.go#L706-L711
        // https://github.com/cosmos/cosmos-sdk/blob/2646b474c7beb0c93d4fafd395ef345f41afc251/types/coin.go#L677
        let full_denom = format!("factory/{}/{}", creator, subdenom);
        if full_denom.len() < 3
            || full_denom.len() > 128
            || creator.as_str().contains('/')
            || subdenom.len() > 44
            || creator.as_str().len() > 75
        {
            return Err(ContractError::InvalidFullDenom { full_denom });
        }
        Ok(full_denom)
    }

    /// Used to mock out the response for TgradeQuery::ValidatorVotes
    pub fn set_pool(&self, storage: &mut dyn Storage, pool_id: u64, pool: &Pool) -> StdResult<()> {
        POOLS.save(storage, pool_id, pool)
    }
}

fn complex_swap(
    storage: &dyn Storage,
    first: Swap,
    route: Vec<Step>,
    amount: SwapAmount,
) -> AnyResult<(SwapAmount, Vec<(u64, Pool)>)> {
    // all the `Swap`s we need to execute in order
    let swaps: Vec<_> = {
        let frst = iter::once(first.clone());
        let rest = iter::once((first.pool_id, first.denom_out))
            .chain(route.into_iter().map(|step| (step.pool_id, step.denom_out)))
            .tuple_windows()
            .map(|((_, denom_in), (pool_id, denom_out))| Swap {
                pool_id,
                denom_in,
                denom_out,
            });
        frst.chain(rest).collect()
    };

    let mut updated_pools = vec![];

    match amount {
        SwapAmount::In(mut input) => {
            for swap in &swaps {
                let mut pool = POOLS.load(storage, swap.pool_id)?;
                let payout = pool.swap(&swap.denom_in, &swap.denom_out, SwapAmount::In(input))?;
                updated_pools.push((swap.pool_id, pool));

                input = payout.as_out();
            }

            Ok((SwapAmount::Out(input), updated_pools))
        }
        SwapAmount::Out(mut output) => {
            for swap in swaps.iter().rev() {
                let mut pool = POOLS.load(storage, swap.pool_id)?;
                let payout = pool.swap(&swap.denom_in, &swap.denom_out, SwapAmount::Out(output))?;
                updated_pools.push((swap.pool_id, pool));

                output = payout.as_in();
            }

            Ok((SwapAmount::In(output), updated_pools))
        }
    }
}

impl Module for OsmosisModule {
    type ExecT = OsmosisMsg;
    type QueryT = OsmosisQuery;
    type SudoT = Empty;

    // Builds a mock rust implementation of the expected osmosis functionality for testing
    fn execute<ExecC, QueryC>(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        sender: Addr,
        msg: OsmosisMsg,
    ) -> AnyResult<AppResponse>
    where
        ExecC: Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
        QueryC: CustomQuery + DeserializeOwned + 'static,
    {
        match msg {
            OsmosisMsg::CreateDenom { subdenom } => {
                // TODO: Simulate denom creation, and add existence checks in MintTokens
                let denom = self.build_denom(&sender, &subdenom)?;
                let data = Some(to_binary(&FullDenomResponse { denom })?);
                Ok(AppResponse {
                    data,
                    events: vec![],
                })
            }
            OsmosisMsg::MintTokens {
                denom,
                amount,
                mint_to_address,
            } => {
                // TODO: This currently incorrectly simulates the Osmosis functionality, as it does not
                // check admin functionality on the denom / that the denom was actually created
                let mint = BankSudo::Mint {
                    to_address: mint_to_address,
                    amount: coins(amount.u128(), &denom),
                };
                router.sudo(api, storage, block, mint.into())?;

                let data = Some(to_binary(&FullDenomResponse { denom })?);
                Ok(AppResponse {
                    data,
                    events: vec![],
                })
            }
            OsmosisMsg::BurnTokens {
                denom: _,
                amount: _,
                burn_from_address: _,
            } => Ok(AppResponse {
                data: None,
                events: vec![],
            }),
            OsmosisMsg::ChangeAdmin {
                denom: _denom,
                new_admin_address: _new_admin_address,
            } => Ok(AppResponse {
                data: None,
                events: vec![],
            }),
            OsmosisMsg::Swap {
                first,
                route,
                amount,
            } => {
                let denom_in = first.denom_in.clone();
                let denom_out = route
                    .iter()
                    .last()
                    .map(|step| step.denom_out.clone())
                    .unwrap_or_else(|| first.denom_out.clone());

                let (swap_result, updated_pools) =
                    complex_swap(storage, first, route, amount.clone().discard_limit())?;

                match amount {
                    SwapAmountWithLimit::ExactIn { min_output, .. } => {
                        if swap_result.as_out() < min_output {
                            return Err(OsmosisError::PriceTooLow.into());
                        }
                    }
                    SwapAmountWithLimit::ExactOut { max_input, .. } => {
                        if swap_result.as_in() > max_input {
                            return Err(OsmosisError::PriceTooLow.into());
                        }
                    }
                }

                for (pool_id, pool) in updated_pools {
                    POOLS.save(storage, pool_id, &pool)?;
                }

                let (pay_in, get_out) = match amount {
                    SwapAmountWithLimit::ExactIn { input, .. } => (input, swap_result.as_out()),
                    SwapAmountWithLimit::ExactOut { output, .. } => (swap_result.as_in(), output),
                };

                // Note: to make testing easier, we just mint and burn - no balance for AMM
                // burn pay_in tokens from sender
                let burn = BankMsg::Burn {
                    amount: coins(pay_in.u128(), &denom_in),
                };
                router.execute(api, storage, block, sender.clone(), burn.into())?;

                // mint get_out tokens to sender
                let mint = BankSudo::Mint {
                    to_address: sender.to_string(),
                    amount: coins(get_out.u128(), denom_out),
                };
                router.sudo(api, storage, block, mint.into())?;

                let output = match amount {
                    SwapAmountWithLimit::ExactIn { .. } => SwapAmount::Out(get_out),
                    SwapAmountWithLimit::ExactOut { .. } => SwapAmount::In(pay_in),
                };
                let data = Some(to_binary(&SwapResponse { amount: output })?);
                Ok(AppResponse {
                    data,
                    events: vec![],
                })
            }
        }
    }

    fn sudo<ExecC, QueryC>(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        _msg: Self::SudoT,
    ) -> AnyResult<AppResponse>
    where
        ExecC: Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
        QueryC: CustomQuery + DeserializeOwned + 'static,
    {
        bail!("sudo not implemented for OsmosisModule")
    }

    fn query(
        &self,
        api: &dyn Api,
        storage: &dyn Storage,
        _querier: &dyn Querier,
        _block: &BlockInfo,
        request: OsmosisQuery,
    ) -> anyhow::Result<Binary> {
        match request {
            OsmosisQuery::FullDenom {
                creator_addr,
                subdenom,
            } => {
                let contract = api.addr_validate(&creator_addr)?;
                let denom = self.build_denom(&contract, &subdenom)?;
                let res = FullDenomResponse { denom };
                Ok(to_binary(&res)?)
            }
            OsmosisQuery::PoolState { id } => {
                let pool = POOLS.load(storage, id)?;
                let res = pool.into_response(id);
                Ok(to_binary(&res)?)
            }
            OsmosisQuery::SpotPrice {
                swap,
                with_swap_fee,
            } => {
                let pool = POOLS.load(storage, swap.pool_id)?;
                let price = pool.spot_price(&swap.denom_in, &swap.denom_out, with_swap_fee)?;
                Ok(to_binary(&SpotPriceResponse { price })?)
            }
            OsmosisQuery::EstimateSwap {
                sender: _sender,
                first,
                route,
                amount,
            } => {
                let (amount, _) = complex_swap(storage, first, route, amount)?;

                Ok(to_binary(&SwapResponse { amount })?)
            }
        }
    }
}

#[derive(Error, Debug, PartialEq)]
pub enum OsmosisError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Asset not in pool")]
    AssetNotInPool,

    #[error("Price under minimum requested, aborting swap")]
    PriceTooLow,

    /// Remove this to let the compiler find all TODOs
    #[error("Not yet implemented (TODO)")]
    Unimplemented,
}

pub type OsmosisAppWrapped =
    App<BankKeeper, MockApi, MockStorage, OsmosisModule, WasmKeeper<OsmosisMsg, OsmosisQuery>>;

pub struct OsmosisApp(OsmosisAppWrapped);

impl Deref for OsmosisApp {
    type Target = OsmosisAppWrapped;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for OsmosisApp {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Querier for OsmosisApp {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        self.0.raw_query(bin_request)
    }
}

impl Default for OsmosisApp {
    fn default() -> Self {
        Self::new()
    }
}

impl OsmosisApp {
    pub fn new() -> Self {
        Self(
            BasicAppBuilder::<OsmosisMsg, OsmosisQuery>::new_custom()
                .with_custom(OsmosisModule {})
                .build(|_router, _, _storage| {
                    // router.custom.set_owner(storage, &owner).unwrap();
                }),
        )
    }

    pub fn block_info(&self) -> BlockInfo {
        self.0.block_info()
    }

    /// This advances BlockInfo by given number of blocks.
    /// It does not do any callbacks, but keeps the ratio of seconds/blokc
    pub fn advance_blocks(&mut self, blocks: u64) {
        self.update_block(|block| {
            block.time = block.time.plus_seconds(BLOCK_TIME * blocks);
            block.height += blocks;
        });
    }

    /// This advances BlockInfo by given number of seconds.
    /// It does not do any callbacks, but keeps the ratio of seconds/blokc
    pub fn advance_seconds(&mut self, seconds: u64) {
        self.update_block(|block| {
            block.time = block.time.plus_seconds(seconds);
            block.height += max(1, seconds / BLOCK_TIME);
        });
    }

    /// Simple iterator when you don't care too much about the details and just want to
    /// simulate forward motion.
    pub fn next_block(&mut self) {
        self.advance_blocks(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::MOCK_CONTRACT_ADDR;
    use cosmwasm_std::{coin, from_slice, Uint128};
    use cw_multi_test::Executor;
    use osmo_bindings::{Step, Swap};

    #[test]
    fn mint_token() {
        let contract = Addr::unchecked("govner");
        let rcpt = Addr::unchecked("townies");
        let subdenom = "fundz";

        let mut app = OsmosisApp::new();

        // no tokens
        let start = app.wrap().query_all_balances(rcpt.as_str()).unwrap();
        assert_eq!(start, vec![]);

        // let's find the mapping
        let FullDenomResponse { denom } = app
            .wrap()
            .query(
                &OsmosisQuery::FullDenom {
                    creator_addr: contract.to_string(),
                    subdenom: subdenom.to_string(),
                }
                .into(),
            )
            .unwrap();
        assert_ne!(denom, subdenom);
        assert!(denom.len() > 10);

        // prepare to mint
        let amount = Uint128::new(1234567);
        let msg = OsmosisMsg::MintTokens {
            denom: denom.to_string(),
            amount,
            mint_to_address: rcpt.to_string(),
        };

        // simulate contract calling
        // TODO: How is this not erroring, the token isn't created
        app.execute(contract, msg.into()).unwrap();

        // we got tokens!
        let end = app.wrap().query_balance(rcpt.as_str(), &denom).unwrap();
        let expected = Coin { denom, amount };
        assert_eq!(end, expected);

        // but no minting of unprefixed version
        let empty = app.wrap().query_balance(rcpt.as_str(), subdenom).unwrap();
        assert_eq!(empty.amount, Uint128::zero());
    }

    #[test]
    fn query_pool() {
        let coin_a = coin(6_000_000u128, "osmo");
        let coin_b = coin(1_500_000u128, "atom");
        let pool_id = 43;
        let pool = Pool::new(coin_a.clone(), coin_b.clone());

        // set up with one pool
        let mut app = OsmosisApp::new();
        app.init_modules(|router, _, storage| {
            router.custom.set_pool(storage, pool_id, &pool).unwrap();
        });

        // query the pool state
        let query = OsmosisQuery::PoolState { id: pool_id }.into();
        let state: PoolStateResponse = app.wrap().query(&query).unwrap();
        let expected_shares = coin(3_000_000, "gamm/pool/43");
        assert_eq!(state.shares, expected_shares);
        assert_eq!(state.assets, vec![coin_a.clone(), coin_b.clone()]);

        // check spot price both directions
        let query = OsmosisQuery::spot_price(pool_id, &coin_a.denom, &coin_b.denom).into();
        let SpotPriceResponse { price } = app.wrap().query(&query).unwrap();
        assert_eq!(price, Decimal::percent(25));

        // and atom -> osmo
        let query = OsmosisQuery::spot_price(pool_id, &coin_b.denom, &coin_a.denom).into();
        let SpotPriceResponse { price } = app.wrap().query(&query).unwrap();
        assert_eq!(price, Decimal::percent(400));

        // with fee
        let query = OsmosisQuery::SpotPrice {
            swap: Swap::new(pool_id, &coin_b.denom, &coin_a.denom),
            with_swap_fee: true,
        };
        let SpotPriceResponse { price } = app.wrap().query(&query.into()).unwrap();
        // 4.00 * (1- 0.3%) = 4 * 0.997 = 3.988
        assert_eq!(price, Decimal::permille(3988));
    }

    #[test]
    fn estimate_swap() {
        let coin_a = coin(6_000_000u128, "osmo");
        let coin_b = coin(1_500_000u128, "atom");
        let pool_id = 43;
        let pool = Pool::new(coin_a.clone(), coin_b.clone());

        // set up with one pool
        let mut app = OsmosisApp::new();
        app.init_modules(|router, _, storage| {
            router.custom.set_pool(storage, pool_id, &pool).unwrap();
        });

        // estimate the price (501505 * 0.997 = 500_000) after fees gone
        let query = OsmosisQuery::estimate_swap(
            MOCK_CONTRACT_ADDR,
            pool_id,
            &coin_b.denom,
            &coin_a.denom,
            SwapAmount::In(Uint128::new(501505)),
        );
        let SwapResponse { amount } = app.wrap().query(&query.into()).unwrap();
        // 6M * 1.5M = 2M * 4.5M -> output = 1.5M
        let expected = SwapAmount::Out(Uint128::new(1_500_000));
        assert_eq!(amount, expected);

        // now try the reverse query. we know what we need to pay to get 1.5M out
        let query = OsmosisQuery::estimate_swap(
            MOCK_CONTRACT_ADDR,
            pool_id,
            &coin_b.denom,
            &coin_a.denom,
            SwapAmount::Out(Uint128::new(1500000)),
        );
        let SwapResponse { amount } = app.wrap().query(&query.into()).unwrap();
        let expected = SwapAmount::In(Uint128::new(501505));
        assert_eq!(amount, expected);
    }

    #[test]
    fn perform_swap() {
        let coin_a = coin(6_000_000u128, "osmo");
        let coin_b = coin(1_500_000u128, "atom");
        let pool_id = 43;
        let pool = Pool::new(coin_a.clone(), coin_b.clone());
        let trader = Addr::unchecked("trader");

        // set up with one pool
        let mut app = OsmosisApp::new();
        app.init_modules(|router, _, storage| {
            router.custom.set_pool(storage, pool_id, &pool).unwrap();
            router
                .bank
                .init_balance(storage, &trader, coins(800_000, &coin_b.denom))
                .unwrap()
        });

        // check balance before
        let Coin { amount, .. } = app.wrap().query_balance(&trader, &coin_a.denom).unwrap();
        assert_eq!(amount, Uint128::new(0));
        let Coin { amount, .. } = app.wrap().query_balance(&trader, &coin_b.denom).unwrap();
        assert_eq!(amount, Uint128::new(800_000));

        // this is too low a payment, will error
        let msg = OsmosisMsg::simple_swap(
            pool_id,
            &coin_b.denom,
            &coin_a.denom,
            SwapAmountWithLimit::ExactOut {
                output: Uint128::new(1_500_000),
                max_input: Uint128::new(400_000),
            },
        );
        let err = app.execute(trader.clone(), msg.into()).unwrap_err();
        println!("{:?}", err);

        // now a proper swap
        let msg = OsmosisMsg::simple_swap(
            pool_id,
            &coin_b.denom,
            &coin_a.denom,
            SwapAmountWithLimit::ExactOut {
                output: Uint128::new(1_500_000),
                max_input: Uint128::new(600_000),
            },
        );
        let res = app.execute(trader.clone(), msg.into()).unwrap();

        // update balances (800_000 - 501_505 paid = 298_495)
        let Coin { amount, .. } = app.wrap().query_balance(&trader, &coin_a.denom).unwrap();
        assert_eq!(amount, Uint128::new(1_500_000));
        let Coin { amount, .. } = app.wrap().query_balance(&trader, &coin_b.denom).unwrap();
        assert_eq!(amount, Uint128::new(298_495));

        // check the response contains proper value
        let input: SwapResponse = from_slice(res.data.unwrap().as_slice()).unwrap();
        assert_eq!(input.amount, SwapAmount::In(Uint128::new(501_505)));

        // check pool state properly updated with fees
        let query = OsmosisQuery::PoolState { id: pool_id }.into();
        let state: PoolStateResponse = app.wrap().query(&query).unwrap();
        let expected_assets = vec![
            coin(4_500_000, &coin_a.denom),
            coin(2_001_505, &coin_b.denom),
        ];
        assert_eq!(state.assets, expected_assets);
    }

    #[test]
    fn swap_with_route_max_input_exceeded() {
        let pool1 = Pool::new(coin(6_000_000, "osmo"), coin(3_000_000, "atom"));
        let pool2 = Pool::new(coin(2_000_000, "atom"), coin(1_000_000, "btc"));
        let trader = Addr::unchecked("trader");

        let mut app = OsmosisApp::new();
        app.init_modules(|router, _, storage| {
            router.custom.set_pool(storage, 1, &pool1).unwrap();
            router.custom.set_pool(storage, 2, &pool2).unwrap();
            router
                .bank
                .init_balance(storage, &trader, coins(5000, "osmo"))
                .unwrap()
        });

        let msg = OsmosisMsg::Swap {
            first: Swap {
                pool_id: 1,
                denom_in: "osmo".to_string(),
                denom_out: "atom".to_string(),
            },
            route: vec![Step {
                pool_id: 2,
                denom_out: "btc".to_string(),
            }],
            amount: SwapAmountWithLimit::ExactOut {
                output: Uint128::new(1000),
                max_input: Uint128::new(4000),
            },
        };
        let err = app.execute(trader, msg.into()).unwrap_err();
        assert_eq!(
            err.downcast::<OsmosisError>().unwrap(),
            OsmosisError::PriceTooLow
        );
    }

    #[test]
    fn swap_with_route_min_output_not_met() {
        let pool1 = Pool::new(coin(6_000_000, "osmo"), coin(3_000_000, "atom"));
        let pool2 = Pool::new(coin(2_000_000, "atom"), coin(1_000_000, "btc"));
        let trader = Addr::unchecked("trader");

        let mut app = OsmosisApp::new();
        app.init_modules(|router, _, storage| {
            router.custom.set_pool(storage, 1, &pool1).unwrap();
            router.custom.set_pool(storage, 2, &pool2).unwrap();
            router
                .bank
                .init_balance(storage, &trader, coins(5000, "osmo"))
                .unwrap()
        });

        let msg = OsmosisMsg::Swap {
            first: Swap {
                pool_id: 1,
                denom_in: "osmo".to_string(),
                denom_out: "atom".to_string(),
            },
            route: vec![Step {
                pool_id: 2,
                denom_out: "btc".to_string(),
            }],
            amount: SwapAmountWithLimit::ExactIn {
                input: Uint128::new(4000),
                min_output: Uint128::new(1000),
            },
        };
        let err = app.execute(trader, msg.into()).unwrap_err();
        assert_eq!(
            err.downcast::<OsmosisError>().unwrap(),
            OsmosisError::PriceTooLow
        );
    }

    #[test]
    fn swap_with_route_wrong_denom() {
        let pool1 = Pool::new(coin(6_000_000, "osmo"), coin(3_000_000, "atom"));
        let pool2 = Pool::new(coin(2_000_000, "atom"), coin(1_000_000, "eth"));
        let trader = Addr::unchecked("trader");

        let mut app = OsmosisApp::new();
        app.init_modules(|router, _, storage| {
            router.custom.set_pool(storage, 1, &pool1).unwrap();
            router.custom.set_pool(storage, 2, &pool2).unwrap();
            router
                .bank
                .init_balance(storage, &trader, coins(5000, "osmo"))
                .unwrap()
        });

        let msg = OsmosisMsg::Swap {
            first: Swap {
                pool_id: 1,
                denom_in: "osmo".to_string(),
                denom_out: "atom".to_string(),
            },
            route: vec![Step {
                pool_id: 2,
                denom_out: "btc".to_string(),
            }],
            amount: SwapAmountWithLimit::ExactOut {
                output: Uint128::new(1000),
                max_input: Uint128::new(4000),
            },
        };
        let err = app.execute(trader, msg.into()).unwrap_err();
        assert_eq!(
            err.downcast::<OsmosisError>().unwrap(),
            OsmosisError::AssetNotInPool
        );
    }

    #[test]
    fn perform_swap_with_route_exact_out() {
        let pool1 = Pool::new(coin(6_000_000, "osmo"), coin(3_000_000, "atom"));
        let pool2 = Pool::new(coin(2_000_000, "atom"), coin(1_000_000, "btc"));
        let trader = Addr::unchecked("trader");

        // set up pools
        let mut app = OsmosisApp::new();
        app.init_modules(|router, _, storage| {
            router.custom.set_pool(storage, 1, &pool1).unwrap();
            router.custom.set_pool(storage, 2, &pool2).unwrap();
            router
                .bank
                .init_balance(storage, &trader, coins(5000, "osmo"))
                .unwrap()
        });

        let msg = OsmosisMsg::Swap {
            first: Swap {
                pool_id: 1,
                denom_in: "osmo".to_string(),
                denom_out: "atom".to_string(),
            },
            route: vec![Step {
                pool_id: 2,
                denom_out: "btc".to_string(),
            }],
            amount: SwapAmountWithLimit::ExactOut {
                output: Uint128::new(1000),
                max_input: Uint128::new(5000),
            },
        };
        let res = app.execute(trader.clone(), msg.into()).unwrap();

        let Coin { amount, .. } = app.wrap().query_balance(&trader, "osmo").unwrap();
        assert_eq!(amount, Uint128::new(5000 - 4033));
        let Coin { amount, .. } = app.wrap().query_balance(&trader, "btc").unwrap();
        assert_eq!(amount, Uint128::new(1000));

        // check the response contains proper value
        let input: SwapResponse = from_slice(res.data.unwrap().as_slice()).unwrap();
        assert_eq!(input.amount, SwapAmount::In(Uint128::new(4033)));

        // check pool state properly updated with fees
        let query = OsmosisQuery::PoolState { id: 1 }.into();
        let state: PoolStateResponse = app.wrap().query(&query).unwrap();
        let expected_assets = vec![
            coin(6_000_000 + 4033, "osmo"),
            coin(3_000_000 - 2009, "atom"),
        ];
        assert_eq!(state.assets, expected_assets);

        let query = OsmosisQuery::PoolState { id: 2 }.into();
        let state: PoolStateResponse = app.wrap().query(&query).unwrap();
        let expected_assets = vec![
            coin(2_000_000 + 2009, "atom"),
            coin(1_000_000 - 1000, "btc"),
        ];
        assert_eq!(state.assets, expected_assets);
    }

    #[test]
    fn perform_swap_with_route_exact_in() {
        let pool1 = Pool::new(coin(6_000_000, "osmo"), coin(3_000_000, "atom"));
        let pool2 = Pool::new(coin(2_000_000, "atom"), coin(1_000_000, "btc"));
        let trader = Addr::unchecked("trader");

        // set up pools
        let mut app = OsmosisApp::new();
        app.init_modules(|router, _, storage| {
            router.custom.set_pool(storage, 1, &pool1).unwrap();
            router.custom.set_pool(storage, 2, &pool2).unwrap();
            router
                .bank
                .init_balance(storage, &trader, coins(5000, "osmo"))
                .unwrap()
        });

        // now a proper swap
        let msg = OsmosisMsg::Swap {
            first: Swap {
                pool_id: 1,
                denom_in: "osmo".to_string(),
                denom_out: "atom".to_string(),
            },
            route: vec![Step {
                pool_id: 2,
                denom_out: "btc".to_string(),
            }],
            amount: SwapAmountWithLimit::ExactIn {
                input: Uint128::new(4000),
                min_output: Uint128::new(900),
            },
        };
        let res = app.execute(trader.clone(), msg.into()).unwrap();

        let Coin { amount, .. } = app.wrap().query_balance(&trader, "osmo").unwrap();
        assert_eq!(amount, Uint128::new(5000 - 4000));
        let Coin { amount, .. } = app.wrap().query_balance(&trader, "btc").unwrap();
        assert_eq!(amount, Uint128::new(993));

        // check the response contains proper value
        let input: SwapResponse = from_slice(res.data.unwrap().as_slice()).unwrap();
        assert_eq!(input.amount, SwapAmount::Out(Uint128::new(993)));

        // check pool state properly updated with fees
        let query = OsmosisQuery::PoolState { id: 1 }.into();
        let state: PoolStateResponse = app.wrap().query(&query).unwrap();
        let expected_assets = vec![
            coin(6_000_000 + 4000, "osmo"),
            coin(3_000_000 - 1993, "atom"),
        ];
        assert_eq!(state.assets, expected_assets);

        let query = OsmosisQuery::PoolState { id: 2 }.into();
        let state: PoolStateResponse = app.wrap().query(&query).unwrap();
        let expected_assets = vec![coin(2_000_000 + 1993, "atom"), coin(1_000_000 - 993, "btc")];
        assert_eq!(state.assets, expected_assets);
    }

    // TODO: make the following test work
    #[test]
    #[ignore]
    fn estimate_swap_regression() {
        let pool = Pool::new(coin(2_000_000, "atom"), coin(1_000_000, "btc"));

        // set up with one pool
        let mut app = OsmosisApp::new();
        app.init_modules(|router, _, storage| {
            router.custom.set_pool(storage, 1, &pool).unwrap();
        });

        // estimate the price (501505 * 0.997 = 500_000) after fees gone
        let query = OsmosisQuery::estimate_swap(
            MOCK_CONTRACT_ADDR,
            1,
            "atom",
            "btc",
            SwapAmount::In(Uint128::new(2007)),
        );
        let SwapResponse { amount } = app.wrap().query(&query.into()).unwrap();
        // 6M * 1.5M = 2M * 4.5M -> output = 1.5M
        let expected = SwapAmount::Out(Uint128::new(1000));
        assert_eq!(amount, expected);

        // now try the reverse query. we know what we need to pay to get 1.5M out
        let query = OsmosisQuery::estimate_swap(
            MOCK_CONTRACT_ADDR,
            1,
            "atom",
            "btc",
            SwapAmount::Out(Uint128::new(1000)),
        );
        let SwapResponse { amount } = app.wrap().query(&query.into()).unwrap();
        let expected = SwapAmount::In(Uint128::new(2007));
        assert_eq!(amount, expected);
    }
}
