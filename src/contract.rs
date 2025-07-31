use cosmwasm_std::{
    entry_point, to_json_binary, BankMsg, Binary, Coin, Deps, DepsMut, Env, Event, MessageInfo, Response, StdError, StdResult
};

use crate::helpers::{create_response, validate_identifier};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, BalanceResponse, AccountResponse, EscrowResponse};
use crate::state::{TOKEN_DENOM, ACCOUNTS, ESCROWS, Escrow};

// version info for migration info
const _CONTRACT_NAME: &str = "crates.io:social-tip-contract";
const _CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    TOKEN_DENOM.save(deps.storage, &msg.token_denom)?;
    Ok(create_response("instantiate", vec![("token_denom", &msg.token_denom)]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: &ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::Register { identifier } => execute_register(deps, info, identifier),
        ExecuteMsg::Transfer { identifier, amount } => execute_transfer(deps, env, info, identifier, amount),
        ExecuteMsg::Claim { identifier } => execute_claim(deps, info, identifier),    
    }
}

fn execute_register(
    deps: DepsMut,
    info: MessageInfo,
    identifier: &str
) -> StdResult<Response> {
    // Validate identifier (basic mail or username check)
    validate_identifier(identifier)?;

    // check if identifier is already registered
    if ACCOUNTS.has(deps.storage, identifier.to_string()) {
        return Err(StdError::generic_err("Identifier already registered"));
    }
    // save mapping
    ACCOUNTS.save(deps.storage, identifier.to_string(), &info.sender)?;
    Ok(create_response(
        "register",
        vec![
            ("identifier", identifier),
            ("address", info.sender.as_ref()),
        ],
    ))
}

fn execute_transfer(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    identifier: &str,
    amount: &Coin,
) -> StdResult<Response> {
    // Validate identifier and token denomination
    validate_identifier(identifier)?;
    let token_denom = TOKEN_DENOM.load(deps.storage)?;
    if amount.denom != token_denom {
        return Err(StdError::generic_err("Invalid token denomination"));
    }
    // Check if recipient is registered
    match ACCOUNTS.may_load(deps.storage, identifier.to_string())? {
        Some(recipient_addr) => {
            // Transfer tokens directly
            let transfer_msg = BankMsg::Send { 
                to_address: recipient_addr.to_string(),
                amount: vec![amount.clone()], 
            };
            Ok(create_response(
                "transfer",
                vec![
                    ("sender", info.sender.as_ref()),
                    ("recipient", identifier),
                    ("amount", &amount.amount.to_string()),
                ],
            ).add_message(transfer_msg))
        }
        None => {
            // Hold token in escrow and emit event for off-chain notification
            let escrow = Escrow {
                sender: info.sender.clone(),
                amount: amount.clone(),
            };
            ESCROWS.save(deps.storage, identifier.to_string(), &escrow)?;
            let event = Event::new("unregistered transfer")
                    .add_attribute("identifier", identifier)
                    .add_attribute("sender", info.sender.to_string())
                    .add_attribute("amount", amount.amount.to_string())
                    .add_attribute("denom", amount.denom.clone());
                Ok(create_response(
                    "escrow",
                    vec![
                        ("identifier", identifier),
                        ("sender", info.sender.as_ref()),
                        ("amount", &amount.amount.to_string()),
                    ],
                ).add_event(event))       
        }
    }
}

fn execute_claim(
    deps: DepsMut,
    info: MessageInfo,
    identifier: &str
) -> StdResult<Response> {
    // validate identifier
    validate_identifier(identifier)?;
    // check if Identifier is registered to the caller
    match ACCOUNTS.may_load(deps.storage, identifier.to_string())? {
        Some(addr) if addr == info.sender => {
            // check for escrowed tokens
            match ESCROWS.may_load(deps.storage, identifier.to_string())? {
                Some(escrow) => {
                    // Transfer escrow tokens
                    let transfer_msg = BankMsg::Send {
                        to_address: info.sender.to_string(),
                        amount: vec![escrow.amount.clone()],
                    };
                    // remove escrow
                    ESCROWS.remove(deps.storage, identifier.to_string());
                    Ok(create_response(
                        "claim",
                        vec![
                            ("identifier", identifier),
                            ("recipient", info.sender.as_ref()),
                            ("amount", &escrow.amount.amount.to_string()),
                        ],
                    ).add_message(transfer_msg))   
                }
                None => Err(StdError::generic_err("No escrowed tokens found")),
            }
        }
        _ => Err(StdError::generic_err("identifier is not registered to caller")),
    }
}

#[entry_point]
pub fn query(
    deps: Deps,
    _env: Env,
    msg: QueryMsg,
) -> StdResult<Binary>{
    match msg {
        QueryMsg::GetBalance { address } => to_json_binary(&query_balance(deps, address)?),
        QueryMsg::GetEscrow { identifier } => to_json_binary(&query_escrow(deps, identifier)?),
        QueryMsg::GetAccount { identifier } =>  to_json_binary(&query_account(deps, identifier)?),
    }
}

fn query_balance(
    deps: Deps,
    address: String
) -> StdResult<BalanceResponse> {
    let addr = deps.api.addr_validate(&address)?;
    let balance = deps.querier.query_balance(addr, TOKEN_DENOM.load(deps.storage)?)?;
    Ok(BalanceResponse { balance })
}

fn query_escrow(
    deps: Deps,
    identifier: String,
) -> StdResult<EscrowResponse> {
    let escrow = ESCROWS.may_load(deps.storage, identifier)?;
    Ok(EscrowResponse { escrow })
}

fn query_account(
    deps: Deps,
    identifier: String,
) -> StdResult<AccountResponse> {
    let address = ACCOUNTS.may_load(deps.storage, identifier)?;
    Ok(AccountResponse { address }) 
}

