use anyhow::{bail, Result as AnyResult};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::cmp::max;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use thiserror::Error;

use cosmwasm_std::testing::{MockApi, MockStorage};
use cosmwasm_std::{
    coins, to_binary, Addr, Api, BankMsg, Binary, BlockInfo, Coin, CustomQuery, Decimal, Empty,
    Isqrt, Querier, QuerierResult, StdError, StdResult, Storage, Uint128,
};
use cw_multi_test::{
    App, AppResponse, BankKeeper, BankSudo, BasicAppBuilder, CosmosRouter, Module, WasmKeeper,
};
use cw_storage_plus::Map;

use osmo_bindings::{
    EstimatePriceResponse, FullDenomResponse, OsmosisMsg, OsmosisQuery, PoolStateResponse,
    SpotPriceResponse, SwapAmount, SwapAmountWithLimit,
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
                let output_minus_fee = output * (Decimal::one() - self.fee);
                let final_in = bal_in * bal_out / (bal_out + output_minus_fee);
                let payout = SwapAmount::In(bal_in - final_in);
                let final_out = bal_out + output;
                (final_in, final_out, payout)
            }
        };
        // update internal balance
        self.set_amount(denom_in, final_in)?;
        self.set_amount(denom_out, final_out)?;
        Ok(payout)
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
    fn build_denom(&self, contract: &Addr, subdenom: &str) -> String {
        // TODO: validation assertion on subdenom
        format!("cw/{}/{}", contract, subdenom)
    }

    /// Used to mock out the response for TgradeQuery::ValidatorVotes
    pub fn set_pool(&self, storage: &mut dyn Storage, pool_id: u64, pool: &Pool) -> StdResult<()> {
        POOLS.save(storage, pool_id, pool)
    }
}

impl Module for OsmosisModule {
    type ExecT = OsmosisMsg;
    type QueryT = OsmosisQuery;
    type SudoT = Empty;

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
            OsmosisMsg::MintTokens {
                subdenom,
                amount,
                recipient,
            } => {
                let denom = self.build_denom(&sender, &subdenom);
                let mint = BankSudo::Mint {
                    to_address: recipient,
                    amount: vec![Coin { denom, amount }],
                };
                router.sudo(api, storage, block, mint.into())
            }
            OsmosisMsg::Swap {
                first,
                route,
                amount,
            } => {
                if !route.is_empty() {
                    return Err(OsmosisError::Unimplemented.into());
                }
                let mut pool = POOLS.load(storage, first.pool_id)?;
                let (pay_in, get_out) = match amount {
                    SwapAmountWithLimit::ExactIn { input, min_output } => {
                        let payout = pool
                            .swap(&first.denom_in, &first.denom_out, SwapAmount::In(input))?
                            .as_out();
                        if payout < min_output {
                            Err(OsmosisError::PriceTooLow)
                        } else {
                            Ok((input, payout))
                        }
                    }
                    SwapAmountWithLimit::ExactOut { output, max_input } => {
                        let payin = pool
                            .swap(&first.denom_in, &first.denom_out, SwapAmount::Out(output))?
                            .as_in();
                        if payin > max_input {
                            Err(OsmosisError::PriceTooLow)
                        } else {
                            Ok((payin, output))
                        }
                    }
                }?;
                // save updated pool state
                POOLS.save(storage, first.pool_id, &pool)?;

                // Note: to make testing easier, we just mint and burn - no balance for AMM
                // burn pay_in tokens from sender
                let burn = BankMsg::Burn {
                    amount: coins(pay_in.u128(), &first.denom_in),
                };
                router.execute(api, storage, block, sender.clone(), burn.into())?;

                // mint get_out tokens to sender
                let mint = BankSudo::Mint {
                    to_address: sender.to_string(),
                    amount: coins(get_out.u128(), &first.denom_out),
                };
                router.sudo(api, storage, block, mint.into())
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
            OsmosisQuery::FullDenom { contract, subdenom } => {
                let contract = api.addr_validate(&contract)?;
                let denom = self.build_denom(&contract, &subdenom);
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
            OsmosisQuery::EstimatePrice {
                first,
                route,
                amount,
            } => {
                if !route.is_empty() {
                    return Err(OsmosisError::Unimplemented.into());
                }
                let mut pool = POOLS.load(storage, first.pool_id)?;
                let amount = pool.swap(&first.denom_in, &first.denom_out, amount)?;
                Ok(to_binary(&EstimatePriceResponse { amount })?)
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
    use cosmwasm_std::{coin, Uint128};
    use cw_multi_test::Executor;
    use osmo_bindings::Swap;

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
                    contract: contract.to_string(),
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
            subdenom: subdenom.to_string(),
            amount,
            recipient: rcpt.to_string(),
        };

        // simulate contract calling
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
}
