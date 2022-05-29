#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128, SubMsg
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, GetDenomResponse, InstantiateMsg, QueryMsg};
use crate::state::{State, STATE};
use osmo_bindings::{OsmosisMsg, OsmosisQuery };

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:tokenfactory-demo";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
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
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<OsmosisMsg>, ContractError> {
    match msg {
        ExecuteMsg::CreateDenom { subdenom } => create_denom(deps, subdenom),
        ExecuteMsg::ChangeAdmin {
            denom,
            new_admin_address,
        } => change_admin(deps, denom, new_admin_address),
        ExecuteMsg::MintTokens {
            denom,
            amount,
            mint_to_address,
        } => mint_tokens(deps, denom, amount, mint_to_address),
        ExecuteMsg::BurnTokens {
            denom,
            amount,
            mint_to_address,
        } => burn_tokens(deps, denom, amount, mint_to_address),
    }
}

pub fn create_denom(deps: DepsMut, subdenom: String) -> Result<Response<OsmosisMsg>, ContractError> {
    let create_denom_msg = OsmosisMsg::CreateDenom{subdenom};

    let res = Response::new()
    .add_attribute("method", "burn_tokens")
    .add_message(<OsmosisMsg>::from(create_denom_msg));

    Ok(res)
}

pub fn change_admin(
    deps: DepsMut,
    denom: String,
    new_admin_address: String,
) -> Result<Response<OsmosisMsg>, ContractError> {
    let change_admin_msg = OsmosisMsg::ChangeAdmin{denom, new_admin_address};

    let res = Response::new()
    .add_attribute("method", "burn_tokens")
    .add_message(<OsmosisMsg>::from(change_admin_msg));

    Ok(res)
}

pub fn mint_tokens(
    deps: DepsMut,
    denom: String,
    amount: Uint128,
    mint_to_address: String,
) -> Result<Response<OsmosisMsg>, ContractError> {

    let mint_tokens_msg = OsmosisMsg::MintTokens{denom, amount, mint_to_address};

    let res = Response::new()
    .add_attribute("method", "burn_tokens")
    .add_message(<OsmosisMsg>::from(mint_tokens_msg));

    Ok(res)
}

pub fn burn_tokens(
    deps: DepsMut,
    denom: String,
    amount: Uint128,
    burn_from_address: String,
) -> Result<Response<OsmosisMsg>, ContractError> {

    let burn_token_msg = OsmosisMsg::burn_contract_tokens(denom, amount, burn_from_address);

    let res = Response::new()
        .add_attribute("method", "burn_tokens")
        .add_message(<OsmosisMsg>::from(burn_token_msg));

    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetDenom {
            creator_address,
            subdenom,
        } => to_binary(&get_denom(deps, creator_address, subdenom)?),
    }
}

fn get_denom(deps: Deps, creator_address: String, subdenom: String) -> StdResult<GetDenomResponse> {
    Ok(GetDenomResponse {
        full_denom: String::from("mycustomdenom"),
        owner: String::from("mycustomdenom"),
        short_name: String::from("mycustomdenom"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies_with_balance, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};

    // #[test]
    // fn proper_initialization() {
    //     let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

    //     let msg = InstantiateMsg { count: 17 };
    //     let info = mock_info("creator", &coins(1000, "earth"));

    //     // we can just call .unwrap() to assert this was a success
    //     let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    //     assert_eq!(0, res.messages.len());

    //     // it worked, let's query the state
    //     let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
    //     let value: CountResponse = from_binary(&res).unwrap();
    //     assert_eq!(17, value.count);
    // }

    #[test]
    fn create_denom() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // beneficiary can release it
        let info = mock_info("anyone", &coins(2, "token"));

        let denom_name_to_create = String::from("mycustomdenom");

        let msg = ExecuteMsg::CreateDenom { subdenom: denom_name_to_create };
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // // should increase counter by 1
        // let res = query(deps.as_ref(), mock_env(), QueryMsg::Get {}).unwrap();
        // let value: CountResponse = from_binary(&res).unwrap();
        // assert_eq!(18, value.count);
    }
}
