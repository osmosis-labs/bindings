#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};
use cw2::set_contract_version;

use crate::error::TokenFactoryError;
use crate::msg::{ExecuteMsg, GetDenomResponse, InstantiateMsg, QueryMsg};
use crate::state::{State, STATE};
use osmo_bindings::{OsmosisMsg, OsmosisQuerier, OsmosisQuery};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:tokenfactory-demo";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut<OsmosisQuery>,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, TokenFactoryError> {
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
    deps: DepsMut<OsmosisQuery>,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<OsmosisMsg>, TokenFactoryError> {
    match msg {
        ExecuteMsg::CreateDenom { subdenom } => create_denom(subdenom),
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
            burn_from_address,
        } => burn_tokens(deps, denom, amount, burn_from_address),
    }
}

pub fn create_denom(subdenom: String) -> Result<Response<OsmosisMsg>, TokenFactoryError> {
    if subdenom.eq("") {
        return Err(TokenFactoryError::InvalidSubdenom { subdenom });
    }

    let create_denom_msg = OsmosisMsg::CreateDenom { subdenom };

    let res = Response::new()
        .add_attribute("method", "create_denom")
        .add_message(create_denom_msg);

    Ok(res)
}

pub fn change_admin(
    deps: DepsMut<OsmosisQuery>,
    denom: String,
    new_admin_address: String,
) -> Result<Response<OsmosisMsg>, TokenFactoryError> {
    deps.api.addr_validate(&new_admin_address)?;

    validate_denom(deps, denom.clone())?;

    let change_admin_msg = OsmosisMsg::ChangeAdmin {
        denom,
        new_admin_address,
    };

    let res = Response::new()
        .add_attribute("method", "change_admin")
        .add_message(change_admin_msg);

    Ok(res)
}

pub fn mint_tokens(
    deps: DepsMut<OsmosisQuery>,
    denom: String,
    amount: Uint128,
    mint_to_address: String,
) -> Result<Response<OsmosisMsg>, TokenFactoryError> {
    deps.api.addr_validate(&mint_to_address)?;

    if amount.eq(&Uint128::new(0_u128)) {
        return Result::Err(TokenFactoryError::ZeroAmount {});
    }

    validate_denom(deps, denom.clone())?;

    let mint_tokens_msg = OsmosisMsg::mint_contract_tokens(denom, amount, mint_to_address);

    let res = Response::new()
        .add_attribute("method", "mint_tokens")
        .add_message(mint_tokens_msg);

    Ok(res)
}

pub fn burn_tokens(
    deps: DepsMut<OsmosisQuery>,
    denom: String,
    amount: Uint128,
    burn_from_address: String,
) -> Result<Response<OsmosisMsg>, TokenFactoryError> {
    if !burn_from_address.is_empty() {
        return Result::Err(TokenFactoryError::BurnFromAddressNotSupported {
            address: burn_from_address,
        });
    }

    if amount.eq(&Uint128::new(0_u128)) {
        return Result::Err(TokenFactoryError::ZeroAmount {});
    }

    validate_denom(deps, denom.clone())?;

    let burn_token_msg = OsmosisMsg::burn_contract_tokens(denom, amount, burn_from_address);

    let res = Response::new()
        .add_attribute("method", "burn_tokens")
        .add_message(burn_token_msg);

    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<OsmosisQuery>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetDenom {
            creator_address,
            subdenom,
        } => to_binary(&get_denom(deps, creator_address, subdenom)),
    }
}

fn get_denom(deps: Deps<OsmosisQuery>, creator_addr: String, subdenom: String) -> GetDenomResponse {
    let querier = OsmosisQuerier::new(&deps.querier);
    let response = querier.full_denom(creator_addr, subdenom).unwrap();

    GetDenomResponse {
        denom: response.denom,
    }
}

fn validate_denom(deps: DepsMut<OsmosisQuery>, denom: String) -> Result<(), TokenFactoryError> {
    let denom_to_split = denom.clone();
    let tokenfactory_denom_parts: Vec<&str> = denom_to_split.split('/').collect();

    if tokenfactory_denom_parts.len() != 3 {
        return Result::Err(TokenFactoryError::InvalidDenom {
            denom,
            message: std::format!(
                "denom must have 3 parts separated by /, had {}",
                tokenfactory_denom_parts.len()
            ),
        });
    }

    let prefix = tokenfactory_denom_parts[0];
    let creator_address = tokenfactory_denom_parts[1];
    let subdenom = tokenfactory_denom_parts[2];

    if !prefix.eq_ignore_ascii_case("factory") {
        return Result::Err(TokenFactoryError::InvalidDenom {
            denom,
            message: std::format!("prefix must be 'factory', was {}", prefix),
        });
    }

    // Validate denom by attempting to query for full denom
    let response = OsmosisQuerier::new(&deps.querier)
        .full_denom(String::from(creator_address), String::from(subdenom));
    if response.is_err() {
        return Result::Err(TokenFactoryError::InvalidDenom {
            denom,
            message: response.err().unwrap().to_string(),
        });
    }

    Result::Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{
        mock_env, mock_info, MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR,
    };
    use cosmwasm_std::{
        coins, from_binary, Attribute, ContractResult, CosmosMsg, OwnedDeps, Querier, StdError,
        SystemError, SystemResult,
    };
    use osmo_bindings::OsmosisQuery;
    use osmo_bindings_test::OsmosisApp;
    use std::marker::PhantomData;

    const DENOM_NAME: &str = "mydenom";
    const DENOM_PREFIX: &str = "factory";

    fn mock_dependencies_with_custom_quierier<Q: Querier>(
        querier: Q,
    ) -> OwnedDeps<MockStorage, MockApi, Q, OsmosisQuery> {
        OwnedDeps {
            storage: MockStorage::default(),
            api: MockApi::default(),
            querier,
            custom_query_type: PhantomData,
        }
    }

    fn mock_dependencies_with_query_error(
    ) -> OwnedDeps<MockStorage, MockApi, MockQuerier<OsmosisQuery>, OsmosisQuery> {
        let custom_querier: MockQuerier<OsmosisQuery> =
            MockQuerier::new(&[(MOCK_CONTRACT_ADDR, &[])]).with_custom_handler(|a| match a {
                OsmosisQuery::FullDenom {
                    creator_addr,
                    subdenom,
                } => {
                    let binary_request = to_binary(a).unwrap();

                    if creator_addr.eq("") {
                        return SystemResult::Err(SystemError::InvalidRequest {
                            error: String::from("invalid creator address"),
                            request: binary_request,
                        });
                    }
                    if subdenom.eq("") {
                        return SystemResult::Err(SystemError::InvalidRequest {
                            error: String::from("invalid subdenom"),
                            request: binary_request,
                        });
                    }
                    SystemResult::Ok(ContractResult::Ok(binary_request))
                }
                _ => SystemResult::Err(SystemError::Unknown {}),
            });
        mock_dependencies_with_custom_quierier(custom_querier)
    }

    pub fn mock_dependencies() -> OwnedDeps<MockStorage, MockApi, OsmosisApp, OsmosisQuery> {
        let custom_querier = OsmosisApp::new();
        mock_dependencies_with_custom_quierier(custom_querier)
    }

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(1000, "uosmo"));

        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn query_get_denom() {
        let deps = mock_dependencies();
        let get_denom_query = QueryMsg::GetDenom {
            creator_address: String::from(MOCK_CONTRACT_ADDR),
            subdenom: String::from(DENOM_NAME),
        };
        let response = query(deps.as_ref(), mock_env(), get_denom_query).unwrap();
        let get_denom_response: GetDenomResponse = from_binary(&response).unwrap();
        assert_eq!(
            format!("{}/{}/{}", DENOM_PREFIX, MOCK_CONTRACT_ADDR, DENOM_NAME),
            get_denom_response.denom
        );
    }

    #[test]
    fn msg_create_denom_success() {
        let mut deps = mock_dependencies();

        let subdenom: String = String::from(DENOM_NAME);

        let msg = ExecuteMsg::CreateDenom { subdenom };
        let info = mock_info("creator", &coins(2, "token"));
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        assert_eq!(1, res.messages.len());

        let expected_message = CosmosMsg::from(OsmosisMsg::CreateDenom {
            subdenom: String::from(DENOM_NAME),
        });
        let actual_message = res.messages.get(0).unwrap();
        assert_eq!(expected_message, actual_message.msg);

        assert_eq!(1, res.attributes.len());

        let expected_attribute = Attribute::new("method", "create_denom");
        let actual_attribute = res.attributes.get(0).unwrap();
        assert_eq!(expected_attribute, actual_attribute);

        assert_eq!(res.data.ok_or(0), Err(0));
    }

    #[test]
    fn msg_create_denom_invalid_subdenom() {
        let mut deps = mock_dependencies();

        let subdenom: String = String::from("");

        let msg = ExecuteMsg::CreateDenom { subdenom };
        let info = mock_info("creator", &coins(2, "token"));
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(
            TokenFactoryError::InvalidSubdenom {
                subdenom: String::from("")
            },
            err
        );
    }

    #[test]
    fn msg_change_admin_success() {
        let mut deps = mock_dependencies();

        const NEW_ADMIN_ADDR: &str = "newadmin";

        let info = mock_info("creator", &coins(2, "token"));

        let full_denom_name: &str =
            &format!("{}/{}/{}", DENOM_PREFIX, MOCK_CONTRACT_ADDR, DENOM_NAME)[..];

        let msg = ExecuteMsg::ChangeAdmin {
            denom: String::from(full_denom_name),
            new_admin_address: String::from(NEW_ADMIN_ADDR),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        assert_eq!(1, res.messages.len());

        let expected_message = CosmosMsg::from(OsmosisMsg::ChangeAdmin {
            denom: String::from(full_denom_name),
            new_admin_address: String::from(NEW_ADMIN_ADDR),
        });
        let actual_message = res.messages.get(0).unwrap();
        assert_eq!(expected_message, actual_message.msg);

        assert_eq!(1, res.attributes.len());

        let expected_attribute = Attribute::new("method", "change_admin");
        let actual_attribute = res.attributes.get(0).unwrap();
        assert_eq!(expected_attribute, actual_attribute);

        assert_eq!(res.data.ok_or(0), Err(0));
    }

    #[test]
    fn msg_change_admin_empty_address() {
        let mut deps = mock_dependencies();

        const EMPTY_ADDR: &str = "";

        let info = mock_info("creator", &coins(2, "token"));

        let msg = ExecuteMsg::ChangeAdmin {
            denom: String::from(DENOM_NAME),
            new_admin_address: String::from(EMPTY_ADDR),
        };
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        match err {
            TokenFactoryError::Std(StdError::GenericErr { msg, .. }) => {
                assert!(msg.contains("human address too short"))
            }
            e => panic!("Unexpected error: {:?}", e),
        }
    }

    #[test]
    fn msg_validate_denom_too_many_parts_valid() {
        let mut deps = mock_dependencies();

        // too many parts in denom
        let full_denom_name: &str =
            &format!("{}/{}/{}", DENOM_PREFIX, MOCK_CONTRACT_ADDR, DENOM_NAME)[..];

        validate_denom(deps.as_mut(), String::from(full_denom_name)).unwrap()
    }

    #[test]
    fn msg_change_admin_invalid_denom() {
        let mut deps = mock_dependencies();

        const NEW_ADMIN_ADDR: &str = "newadmin";

        let info = mock_info("creator", &coins(2, "token"));

        // too many parts in denom
        let full_denom_name: &str = &format!(
            "{}/{}/{}/invalid",
            DENOM_PREFIX, MOCK_CONTRACT_ADDR, DENOM_NAME
        )[..];

        let msg = ExecuteMsg::ChangeAdmin {
            denom: String::from(full_denom_name),
            new_admin_address: String::from(NEW_ADMIN_ADDR),
        };
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

        let expected_error = TokenFactoryError::InvalidDenom {
            denom: String::from(full_denom_name),
            message: String::from("denom must have 3 parts separated by /, had 4"),
        };

        assert_eq!(expected_error, err);
    }

    #[test]
    fn msg_mint_tokens_success() {
        let mut deps = mock_dependencies();

        const NEW_ADMIN_ADDR: &str = "newadmin";

        let mint_amount = Uint128::new(100_u128);

        let full_denom_name: &str =
            &format!("{}/{}/{}", DENOM_PREFIX, MOCK_CONTRACT_ADDR, DENOM_NAME)[..];

        let info = mock_info("creator", &coins(2, "token"));

        let msg = ExecuteMsg::MintTokens {
            denom: String::from(full_denom_name),
            amount: mint_amount,
            mint_to_address: String::from(NEW_ADMIN_ADDR),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        assert_eq!(1, res.messages.len());

        let expected_message = CosmosMsg::from(OsmosisMsg::MintTokens {
            denom: String::from(full_denom_name),
            amount: mint_amount,
            mint_to_address: String::from(NEW_ADMIN_ADDR),
        });
        let actual_message = res.messages.get(0).unwrap();
        assert_eq!(expected_message, actual_message.msg);

        assert_eq!(1, res.attributes.len());

        let expected_attribute = Attribute::new("method", "mint_tokens");
        let actual_attribute = res.attributes.get(0).unwrap();
        assert_eq!(expected_attribute, actual_attribute);

        assert_eq!(res.data.ok_or(0), Err(0));
    }

    #[test]
    fn msg_mint_invalid_denom() {
        let mut deps = mock_dependencies();

        const NEW_ADMIN_ADDR: &str = "newadmin";

        let mint_amount = Uint128::new(100_u128);

        let info = mock_info("creator", &coins(2, "token"));

        let full_denom_name: &str = &format!("{}/{}", DENOM_PREFIX, MOCK_CONTRACT_ADDR)[..];
        let msg = ExecuteMsg::MintTokens {
            denom: String::from(full_denom_name),
            amount: mint_amount,
            mint_to_address: String::from(NEW_ADMIN_ADDR),
        };
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        let expected_error = TokenFactoryError::InvalidDenom {
            denom: String::from(full_denom_name),
            message: String::from("denom must have 3 parts separated by /, had 2"),
        };

        assert_eq!(expected_error, err);
    }

    #[test]
    fn msg_burn_tokens_success() {
        let mut deps = mock_dependencies();

        let mint_amount = Uint128::new(100_u128);
        let full_denom_name: &str =
            &format!("{}/{}/{}", DENOM_PREFIX, MOCK_CONTRACT_ADDR, DENOM_NAME)[..];

        let info = mock_info("creator", &coins(2, "token"));

        let msg = ExecuteMsg::BurnTokens {
            denom: String::from(full_denom_name),
            burn_from_address: String::from(""),
            amount: mint_amount,
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        assert_eq!(1, res.messages.len());
        let expected_message = CosmosMsg::from(OsmosisMsg::BurnTokens {
            denom: String::from(full_denom_name),
            amount: mint_amount,
            burn_from_address: String::from(""),
        });
        let actual_message = res.messages.get(0).unwrap();
        assert_eq!(expected_message, actual_message.msg);

        assert_eq!(1, res.attributes.len());

        let expected_attribute = Attribute::new("method", "burn_tokens");
        let actual_attribute = res.attributes.get(0).unwrap();
        assert_eq!(expected_attribute, actual_attribute);

        assert_eq!(res.data.ok_or(0), Err(0))
    }

    #[test]
    fn msg_burn_tokens_input_adddress() {
        let mut deps = mock_dependencies();

        const BURN_FROM_ADDR: &str = "burnfrom";
        let burn_amount = Uint128::new(100_u128);
        let full_denom_name: &str =
            &format!("{}/{}/{}", DENOM_PREFIX, MOCK_CONTRACT_ADDR, DENOM_NAME)[..];

        let info = mock_info("creator", &coins(2, "token"));

        let msg = ExecuteMsg::BurnTokens {
            denom: String::from(full_denom_name),
            burn_from_address: String::from(BURN_FROM_ADDR),
            amount: burn_amount,
        };
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

        let expected_error = TokenFactoryError::BurnFromAddressNotSupported {
            address: String::from(BURN_FROM_ADDR),
        };

        assert_eq!(expected_error, err)
    }

    #[test]
    fn msg_validate_denom_too_many_parts_invalid() {
        let mut deps = mock_dependencies();

        // too many parts in denom
        let full_denom_name: &str = &format!(
            "{}/{}/{}/invalid",
            DENOM_PREFIX, MOCK_CONTRACT_ADDR, DENOM_NAME
        )[..];

        let err = validate_denom(deps.as_mut(), String::from(full_denom_name)).unwrap_err();

        let expected_error = TokenFactoryError::InvalidDenom {
            denom: String::from(full_denom_name),
            message: String::from("denom must have 3 parts separated by /, had 4"),
        };

        assert_eq!(expected_error, err);
    }

    #[test]
    fn msg_validate_denom_not_enough_parts_invalid() {
        let mut deps = mock_dependencies();

        // too little parts in denom
        let full_denom_name: &str = &format!("{}/{}", DENOM_PREFIX, MOCK_CONTRACT_ADDR)[..];

        let err = validate_denom(deps.as_mut(), String::from(full_denom_name)).unwrap_err();

        let expected_error = TokenFactoryError::InvalidDenom {
            denom: String::from(full_denom_name),
            message: String::from("denom must have 3 parts separated by /, had 2"),
        };

        assert_eq!(expected_error, err);
    }

    #[test]
    fn msg_validate_denom_denom_prefix_invalid() {
        let mut deps = mock_dependencies();

        // invalid denom prefix
        let full_denom_name: &str =
            &format!("{}/{}/{}", "invalid", MOCK_CONTRACT_ADDR, DENOM_NAME)[..];

        let err = validate_denom(deps.as_mut(), String::from(full_denom_name)).unwrap_err();

        let expected_error = TokenFactoryError::InvalidDenom {
            denom: String::from(full_denom_name),
            message: String::from("prefix must be 'factory', was invalid"),
        };

        assert_eq!(expected_error, err);
    }

    #[test]
    fn msg_validate_denom_creator_address_invalid() {
        let mut deps = mock_dependencies_with_query_error();

        let full_denom_name: &str = &format!("{}/{}/{}", DENOM_PREFIX, "", DENOM_NAME)[..]; // empty contract address

        let err = validate_denom(deps.as_mut(), String::from(full_denom_name)).unwrap_err();

        match err {
            TokenFactoryError::InvalidDenom { denom, message } => {
                assert_eq!(String::from(full_denom_name), denom);
                assert!(message.contains("invalid creator address"))
            }
            err => panic!("Unexpected error: {:?}", err),
        }
    }
}
