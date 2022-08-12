#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

use crate::error::TwapError;
use crate::msg::{
    GetArithmeticTwapResponse, GetArithmeticTwapToNowResponse, InstantiateMsg, QueryMsg,
};
use crate::state::{State, STATE};
use osmo_bindings::{OsmosisQuerier, OsmosisQuery};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:twap-demo";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut<OsmosisQuery>,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, TwapError> {
    let state = State {
        owner: info.sender.clone(),
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<OsmosisQuery>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetArithmeticTwap {
            id,
            quote_asset_denom,
            base_asset_denom,
            start_time,
            end_time,
        } => to_binary(&get_arithmetic_twap(
            deps,
            id,
            quote_asset_denom,
            base_asset_denom,
            start_time,
            end_time,
        )),

        QueryMsg::GetArithmeticTwapToNow {
            id,
            quote_asset_denom,
            base_asset_denom,
            start_time,
        } => to_binary(&get_arithmetic_twap_to_now(
            deps,
            id,
            quote_asset_denom,
            base_asset_denom,
            start_time,
        )),
    }
}

fn get_arithmetic_twap(
    deps: Deps<OsmosisQuery>,
    id: u64,
    quote_asset_denom: String,
    base_asset_denom: String,
    start_time: i64,
    end_time: i64,
) -> GetArithmeticTwapResponse {
    let querier = OsmosisQuerier::new(&deps.querier);
    let response = querier
        .arithmetic_twap(
            id,
            quote_asset_denom,
            base_asset_denom,
            start_time,
            end_time,
        )
        .unwrap();

    GetArithmeticTwapResponse {
        twap: response.twap,
    }
}

fn get_arithmetic_twap_to_now(
    deps: Deps<OsmosisQuery>,
    id: u64,
    quote_asset_denom: String,
    base_asset_denom: String,
    start_time: i64,
) -> GetArithmeticTwapToNowResponse {
    let querier = OsmosisQuerier::new(&deps.querier);
    let response = querier
        .arithmetic_twap_to_now(id, quote_asset_denom, base_asset_denom, start_time)
        .unwrap();

        GetArithmeticTwapToNowResponse {
        twap: response.twap,
    }
}
