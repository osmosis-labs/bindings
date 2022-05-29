use cosmwasm_std::{
    entry_point, to_binary, to_vec, ContractResult, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    QueryRequest, QueryResponse, Reply, Response, StdError, StdResult, SubMsg, SystemResult,
};
use osmo_bindings::{OsmosisMsg, OsmosisQuery};

use crate::errors::ReflectError;
use crate::msg::{ChainResponse, ExecuteMsg, InstantiateMsg, OwnerResponse, QueryMsg};
use crate::state::{config, config_read, replies, replies_read, State};

#[entry_point]
pub fn instantiate(
    deps: DepsMut<OsmosisQuery>,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response<OsmosisMsg>> {
    let state = State { owner: info.sender };
    config(deps.storage).save(&state)?;
    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut<OsmosisQuery>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<OsmosisMsg>, ReflectError> {
    match msg {
        ExecuteMsg::ReflectMsg { msgs } => execute_reflect(deps, env, info, msgs),
        ExecuteMsg::ReflectSubMsg { msgs } => execute_reflect_subcall(deps, env, info, msgs),
        ExecuteMsg::ChangeOwner { owner } => execute_change_owner(deps, env, info, owner),
    }
}

pub fn execute_reflect(
    deps: DepsMut<OsmosisQuery>,
    _env: Env,
    info: MessageInfo,
    msgs: Vec<CosmosMsg<OsmosisMsg>>,
) -> Result<Response<OsmosisMsg>, ReflectError> {
    let state = config(deps.storage).load()?;

    if info.sender != state.owner {
        return Err(ReflectError::NotCurrentOwner {
            expected: state.owner.into(),
            actual: info.sender.into(),
        });
    }

    if msgs.is_empty() {
        return Err(ReflectError::MessagesEmpty);
    }

    Ok(Response::new()
        .add_attribute("action", "reflect")
        .add_messages(msgs))
}

pub fn execute_reflect_subcall(
    deps: DepsMut<OsmosisQuery>,
    _env: Env,
    info: MessageInfo,
    msgs: Vec<SubMsg<OsmosisMsg>>,
) -> Result<Response<OsmosisMsg>, ReflectError> {
    let state = config(deps.storage).load()?;
    if info.sender != state.owner {
        return Err(ReflectError::NotCurrentOwner {
            expected: state.owner.into(),
            actual: info.sender.into(),
        });
    }

    if msgs.is_empty() {
        return Err(ReflectError::MessagesEmpty);
    }

    Ok(Response::new()
        .add_attribute("action", "reflect_subcall")
        .add_submessages(msgs))
}

pub fn execute_change_owner(
    deps: DepsMut<OsmosisQuery>,
    _env: Env,
    info: MessageInfo,
    new_owner: String,
) -> Result<Response<OsmosisMsg>, ReflectError> {
    let api = deps.api;
    config(deps.storage).update(|mut state| {
        if info.sender != state.owner {
            return Err(ReflectError::NotCurrentOwner {
                expected: state.owner.into(),
                actual: info.sender.into(),
            });
        }
        state.owner = api.addr_validate(&new_owner)?;
        Ok(state)
    })?;
    Ok(Response::new()
        .add_attribute("action", "change_owner")
        .add_attribute("owner", new_owner))
}

/// This just stores the result for future query
#[entry_point]
pub fn reply(deps: DepsMut<OsmosisQuery>, _env: Env, msg: Reply) -> Result<Response, ReflectError> {
    let key = msg.id.to_be_bytes();
    replies(deps.storage).save(&key, &msg)?;
    Ok(Response::default())
}

#[entry_point]
pub fn query(deps: Deps<OsmosisQuery>, _env: Env, msg: QueryMsg) -> StdResult<QueryResponse> {
    match msg {
        QueryMsg::Owner {} => to_binary(&query_owner(deps)?),
        QueryMsg::Chain { request } => to_binary(&query_chain(deps, &request)?),
        QueryMsg::SubMsgResult { id } => to_binary(&query_subcall(deps, id)?),
    }
}

fn query_owner(deps: Deps<OsmosisQuery>) -> StdResult<OwnerResponse> {
    let state = config_read(deps.storage).load()?;
    let resp = OwnerResponse {
        owner: state.owner.into(),
    };
    Ok(resp)
}

fn query_subcall(deps: Deps<OsmosisQuery>, id: u64) -> StdResult<Reply> {
    let key = id.to_be_bytes();
    replies_read(deps.storage).load(&key)
}

fn query_chain(
    deps: Deps<OsmosisQuery>,
    request: &QueryRequest<OsmosisQuery>,
) -> StdResult<ChainResponse> {
    let raw = to_vec(request).map_err(|serialize_err| {
        StdError::generic_err(format!("Serializing QueryRequest: {}", serialize_err))
    })?;
    match deps.querier.raw_query(&raw) {
        SystemResult::Err(system_err) => Err(StdError::generic_err(format!(
            "Querier system error: {}",
            system_err
        ))),
        SystemResult::Ok(ContractResult::Err(contract_err)) => Err(StdError::generic_err(format!(
            "Querier contract error: {}",
            contract_err
        ))),
        SystemResult::Ok(ContractResult::Ok(value)) => Ok(ChainResponse { data: value }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{
        mock_env, mock_info, MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR,
    };
    use cosmwasm_std::{
        coin, coins, from_binary, AllBalanceResponse, BankMsg, BankQuery, Binary, Coin, Event,
        StakingMsg, StdError, SubMsgResponse,
    };
    use cosmwasm_std::{OwnedDeps, SubMsgResult, SystemError};
    use std::marker::PhantomData;

    pub fn mock_dependencies(
        contract_balance: &[Coin],
    ) -> OwnedDeps<MockStorage, MockApi, MockQuerier<OsmosisQuery>, OsmosisQuery> {
        let custom_querier: MockQuerier<OsmosisQuery> =
            MockQuerier::new(&[(MOCK_CONTRACT_ADDR, contract_balance)]).with_custom_handler(|_| {
                SystemResult::Err(SystemError::InvalidRequest {
                    error: "not implemented".to_string(),
                    request: Default::default(),
                })
            });
        OwnedDeps {
            storage: MockStorage::default(),
            api: MockApi::default(),
            querier: custom_querier,
            custom_query_type: PhantomData,
        }
    }

    #[test]
    fn proper_instantialization() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let value = query_owner(deps.as_ref()).unwrap();
        assert_eq!("creator", value.owner.as_str());
    }

    #[test]
    fn reflect() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let payload = vec![BankMsg::Send {
            to_address: String::from("friend"),
            amount: coins(1, "token"),
        }
        .into()];

        let msg = ExecuteMsg::ReflectMsg {
            msgs: payload.clone(),
        };
        let info = mock_info("creator", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let payload: Vec<_> = payload.into_iter().map(SubMsg::new).collect();
        assert_eq!(payload, res.messages);
    }

    #[test]
    fn reflect_requires_owner() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // signer is not owner
        let payload = vec![BankMsg::Send {
            to_address: String::from("friend"),
            amount: coins(1, "token"),
        }
        .into()];
        let msg = ExecuteMsg::ReflectMsg { msgs: payload };

        let info = mock_info("random", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg);
        match res.unwrap_err() {
            ReflectError::NotCurrentOwner { .. } => {}
            err => panic!("Unexpected error: {:?}", err),
        }
    }

    #[test]
    fn reflect_reject_empty_msgs() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("creator", &[]);
        let payload = vec![];

        let msg = ExecuteMsg::ReflectMsg { msgs: payload };
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ReflectError::MessagesEmpty);
    }

    #[test]
    fn reflect_multiple_messages() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let payload = vec![
            BankMsg::Send {
                to_address: String::from("friend"),
                amount: coins(1, "token"),
            }
            .into(),
            StakingMsg::Delegate {
                validator: String::from("validator"),
                amount: coin(100, "ustake"),
            }
            .into(),
        ];

        let msg = ExecuteMsg::ReflectMsg {
            msgs: payload.clone(),
        };
        let info = mock_info("creator", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let payload: Vec<_> = payload.into_iter().map(SubMsg::new).collect();
        assert_eq!(payload, res.messages);
    }

    #[test]
    fn change_owner_works() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("creator", &[]);
        let new_owner = String::from("friend");
        let msg = ExecuteMsg::ChangeOwner { owner: new_owner };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // should change state
        assert_eq!(0, res.messages.len());
        let value = query_owner(deps.as_ref()).unwrap();
        assert_eq!("friend", value.owner.as_str());
    }

    #[test]
    fn change_owner_requires_current_owner_as_sender() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {};
        let creator = String::from("creator");
        let info = mock_info(&creator, &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let random = String::from("random");
        let info = mock_info(&random, &[]);
        let new_owner = String::from("friend");
        let msg = ExecuteMsg::ChangeOwner { owner: new_owner };

        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(
            err,
            ReflectError::NotCurrentOwner {
                expected: creator,
                actual: random
            }
        );
    }

    #[test]
    fn change_owner_errors_for_invalid_new_address() {
        let mut deps = mock_dependencies(&[]);
        let creator = String::from("creator");

        let msg = InstantiateMsg {};
        let info = mock_info(&creator, &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info(&creator, &[]);
        let msg = ExecuteMsg::ChangeOwner {
            owner: String::from("x"),
        };
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        match err {
            ReflectError::Std(StdError::GenericErr { msg, .. }) => {
                assert!(msg.contains("human address too short"))
            }
            e => panic!("Unexpected error: {:?}", e),
        }
    }

    #[test]
    fn chain_query_works() {
        let deps = mock_dependencies(&coins(123, "ucosm"));

        // with bank query
        let msg = QueryMsg::Chain {
            request: BankQuery::AllBalances {
                address: MOCK_CONTRACT_ADDR.to_string(),
            }
            .into(),
        };
        let response = query(deps.as_ref(), mock_env(), msg).unwrap();
        let outer: ChainResponse = from_binary(&response).unwrap();
        let inner: AllBalanceResponse = from_binary(&outer.data).unwrap();
        assert_eq!(inner.amount, coins(123, "ucosm"));

        // TODO? or better in multitest?
        // // with custom query
        // let msg = QueryMsg::Chain {
        //     request: OsmosisQuery::Ping {}.into(),
        // };
        // let response = query(deps.as_ref(), mock_env(), msg).unwrap();
        // let outer: ChainResponse = from_binary(&response).unwrap();
        // let inner: SpecialResponse = from_binary(&outer.data).unwrap();
        // assert_eq!(inner.msg, "pong");
    }

    #[test]
    fn reflect_subcall() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let id = 123u64;
        let payload = SubMsg::reply_always(
            BankMsg::Send {
                to_address: String::from("friend"),
                amount: coins(1, "token"),
            },
            id,
        );

        let msg = ExecuteMsg::ReflectSubMsg {
            msgs: vec![payload.clone()],
        };
        let info = mock_info("creator", &[]);
        let mut res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(1, res.messages.len());
        let msg = res.messages.pop().expect("must have a message");
        assert_eq!(payload, msg);
    }

    // this mocks out what happens after reflect_subcall
    #[test]
    fn reply_and_query() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let id = 123u64;
        let data = Binary::from(b"foobar");
        let events = vec![Event::new("message").add_attribute("signer", "caller-addr")];
        let result = SubMsgResult::Ok(SubMsgResponse {
            events: events.clone(),
            data: Some(data.clone()),
        });
        let subcall = Reply { id, result };
        let res = reply(deps.as_mut(), mock_env(), subcall).unwrap();
        assert_eq!(0, res.messages.len());

        // query for a non-existant id
        let qres = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::SubMsgResult { id: 65432 },
        );
        assert!(qres.is_err());

        // query for the real id
        let raw = query(deps.as_ref(), mock_env(), QueryMsg::SubMsgResult { id }).unwrap();
        let qres: Reply = from_binary(&raw).unwrap();
        assert_eq!(qres.id, id);
        let result = qres.result.unwrap();
        assert_eq!(result.data, Some(data));
        assert_eq!(result.events, events);
    }
}
