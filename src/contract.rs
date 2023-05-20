#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, GetPollResponse, InstantiateMsg, QueryMsg};
use crate::state::{Config, Poll, CONFIG, POLLS};

const CONTRACT_NAME: &str = "crates.io:zero-to-hero";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let validated_admin_address = deps.api.addr_validate(&msg.admin_address)?;
    let config = Config {
        admin_address: validated_admin_address,
    };
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attribute("action", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreatePoll { question } => execute_create_poll(deps, env, info, question),
        ExecuteMsg::Vote { question, choice } => execute_vote(deps, env, info, question, choice),
    }
}
fn execute_vote(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    question: String,
    choice: String,
) -> Result<Response, ContractError> {
    if !POLLS.has(deps.storage, question.clone()) {
        // If there is no poll with the key question, error out saying poll does not exists
        return Err(ContractError::CustomError {
            val: "Poll does not exists".to_string(),
        });
    }
    let mut poll = POLLS.load(deps.storage, question.clone())?;
    if choice != "yes" && choice != "no" {
        return Err(ContractError::CustomError {
            val: "Unrecognized choice".to_string(),
        });
    } else {
        if choice == "yes" {
            poll.yes_votes += 1;
        } else if choice == "no" {
            poll.no_votes += 1
        }
        POLLS.save(deps.storage, question, &poll)?;
        Ok(Response::new().add_attribute("action", "vote"))
    }
}
fn execute_create_poll(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    question: String,
) -> Result<Response, ContractError> {
    // Does the map have a key of this value
    if POLLS.has(deps.storage, question.clone()) {
        // If it does, we want to error!
        return Err(ContractError::CustomError {
            val: "Key already exists".to_string(),
        });
    }
    let poll = Poll {
        question: question.clone(),
        yes_votes: 0,
        no_votes: 0,
    };
    POLLS.save(deps.storage, question, &poll)?;
    Ok(Response::new().add_attribute("action", "create poll"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetPoll { question } => query_get_poll(deps, env, question),
        QueryMsg::GetConfig {} => to_binary(&CONFIG.load(deps.storage)?),
    }
}
fn query_get_poll(deps: Deps, _env: Env, question: String) -> StdResult<Binary> {
    let poll = POLLS.may_load(deps.storage, question)?;
    to_binary(&GetPollResponse { poll })
}
#[cfg(test)]
mod tests {
    use super::{execute, instantiate};
    use crate::contract::query;
    use crate::msg::{ExecuteMsg, GetPollResponse, InstantiateMsg, QueryMsg};
    use crate::state::{Config, Poll};
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{attr, from_binary, Addr};

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("addr1", &[]);
        let msg = InstantiateMsg {
            admin_address: "addr1".to_string(),
        };
        let resp = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(resp.attributes, vec![attr("action", "instantiate")]);

        let msg = QueryMsg::GetConfig {};
        let resp = query(deps.as_ref(), env, msg).unwrap();
        let config: Config = from_binary(&resp).unwrap();
        assert_eq!(
            config,
            Config {
                admin_address: Addr::unchecked("addr1")
            }
        );
    }
    #[test]
    fn test_create_poll() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("addr1", &[]);
        let msg = InstantiateMsg {
            admin_address: "addr1".to_string(),
        };
        let _resp = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        let msg = ExecuteMsg::CreatePoll {
            question: "Do you love Spark IBC?".to_string(),
        };
        let resp = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(resp.attributes, vec![attr("action", "create poll")]);

        let msg = QueryMsg::GetPoll {
            question: "Do you love Spark IBC?".to_string(),
        };
        let resp = query(deps.as_ref(), env.clone(), msg).unwrap();
        let get_poll_response: GetPollResponse = from_binary(&resp).unwrap();
        assert_eq!(
            get_poll_response,
            GetPollResponse {
                poll: Some(Poll {
                    question: "Do you love Spark IBC?".to_string(),
                    yes_votes: 0,
                    no_votes: 0
                })
            }
        );
        let msg = ExecuteMsg::CreatePoll {
            question: "Do you love Spark IBC?".to_string(),
        };
        let _resp = execute(deps.as_mut(), env, info, msg).unwrap_err();
    }
    #[test]
    fn test_vote() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("addr1", &[]);
        let msg = InstantiateMsg {
            admin_address: "addr1".to_string(),
        };
        let _resp = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        let msg = ExecuteMsg::CreatePoll {
            question: "Do you love Spark IBC?".to_string(),
        };
        let _resp = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // Success case, we vote on a poll that exists
        let msg = ExecuteMsg::Vote {
            question: "Do you love Spark IBC?".to_string(),
            choice: "yes".to_string(),
        };
        let resp = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(resp.attributes, vec![attr("action", "vote")]);

        // Success case, we count yes vote & no vote
        let msg = QueryMsg::GetPoll {
            question: "Do you love Spark IBC?".to_string(),
        };
        let resp = query(deps.as_ref(), env.clone(), msg).unwrap();
        let get_poll_response: GetPollResponse = from_binary(&resp).unwrap();
        assert_eq!(
            get_poll_response,
            GetPollResponse {
                poll: Some(Poll {
                    question: "Do you love Spark IBC?".to_string(),
                    yes_votes: 1,
                    no_votes: 0
                })
            }
        );

        // Error case 2: we vote a poll that doesn't exists
        let msg = ExecuteMsg::Vote {
            question: "Do you hate Spark IBC?".to_string(),
            choice: "yes".to_string(),
        };
        let _resp = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();

        // Error case 2: we vote a poll that exists, but with invalid choice
        let msg = ExecuteMsg::Vote {
            question: "Do you love Spark IBC?".to_string(),
            choice: "test".to_string(),
        };
        let _resp = execute(deps.as_mut(), env, info, msg).unwrap_err();
    }
}
