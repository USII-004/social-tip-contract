use cosmwasm_std::{
    entry_point, to_json_binary, BankMsg, Binary, Coin, Deps, DepsMut,
    Env, Event, MessageInfo, Response, StdError, StdResult,
};

use crate::helpers::{create_response, validate_identifier};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, BalanceResponse, AccountResponse, EscrowResponse};
use crate::state::{Config, Escrow, ACCOUNTS, CONFIG, ESCROWS, TOKEN_DENOM};

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
    let config = Config {
        token_denom: msg.token_denom.clone(),
        platform_wallet: msg.platform_wallet.clone(),
    };
    CONFIG.save(deps.storage, &config)?;
    Ok(create_response("instantiate", vec![
        ("token_denom", &msg.token_denom),
        ("platform_wallet", &msg.platform_wallet),
        ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::Register { identifier } => execute_register(deps, info, identifier),
        ExecuteMsg::Transfer { identifier, amount } => execute_transfer(deps, env, info, identifier, &amount),
        ExecuteMsg::Claim { identifier } => execute_claim(deps, info, identifier),    
    }
}

fn execute_register(
    deps: DepsMut,
    info: MessageInfo,
    identifier: String
) -> StdResult<Response> {
    // Validate identifier (basic mail or username check)
    validate_identifier(identifier.clone())?;

    // check if identifier is already registered
    if ACCOUNTS.has(deps.storage, identifier.to_string()) {
        return Err(StdError::generic_err("Identifier already registered"));
    }
    // save mapping
    ACCOUNTS.save(deps.storage, identifier.to_string(), &info.sender)?;
    Ok(create_response(
        "register",
        vec![
            ("identifier", &identifier),
            ("address", info.sender.as_ref()),
        ],
    ))
}

fn execute_transfer(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    identifier: String,
    amount: &Coin,
) -> StdResult<Response> {
    // Validate identifier and token denomination
    validate_identifier(identifier.clone())?;

    let config = CONFIG.load(deps.storage)?;

    if amount.denom != config.token_denom {
        return Err(StdError::generic_err("Invalid token denomination"));
    }

    // implement platfrom fee for transfers
    let platfrom_fee = amount.amount.multiply_ratio(1u128, 100u128);  //1%
    let recipient_amount = {
        match amount.amount.checked_sub(platfrom_fee) {
            Ok(val) => val,
            Err(_) => return Err(StdError::generic_err("Amount too small to cover fees")),
        }
    };

    let fee_msg = BankMsg::Send {
        to_address: config.platform_wallet.clone(),
        amount: vec![Coin {
            denom: amount.denom.clone(),
            amount: platfrom_fee,
        }],
    };

    // Check if recipient is registered
    match ACCOUNTS.may_load(deps.storage, identifier.to_string())? {
        Some(recipient_addr) => {
            // Transfer tokens directly
            let transfer_msg = BankMsg::Send { 
                to_address: recipient_addr.to_string(),
                amount: vec![Coin {
                    denom: amount.denom.clone(),
                    amount: recipient_amount,
                }], 
            };
            Ok(create_response(
                "transfer",
                vec![
                    ("sender", info.sender.as_ref()),
                    ("recipient", &identifier),
                    ("amount", &amount.amount.to_string()),
                    ("fee", &platfrom_fee.to_string()),
                ],
            ).add_messages(vec![transfer_msg, fee_msg]))
        }
        None => {
            // Hold token in escrow and emit event for off-chain notification
            let escrow = Escrow {
                sender: info.sender.clone(),
                amount: Coin {
                    denom: amount.denom.clone(),
                    amount: recipient_amount,
                },
            };
            ESCROWS.save(deps.storage, identifier.to_string(), &escrow)?;
            let event = Event::new("unregistered transfer")
                    .add_attribute("identifier", &identifier)
                    .add_attribute("sender", info.sender.to_string())
                    .add_attribute("amount", amount.amount.to_string())
                    .add_attribute("denom", amount.denom.clone());
                Ok(create_response(
                    "escrow",
                    vec![
                        ("identifier", &identifier),
                        ("sender", info.sender.as_ref()),
                        ("amount", &amount.amount.to_string()),
                        ("fee", &platfrom_fee.to_string()),
                    ],
                )
                .add_event(event)
                .add_message(fee_msg))       
        }
    }
}

fn execute_claim(
    deps: DepsMut,
    info: MessageInfo,
    identifier: String
) -> StdResult<Response> {
    // validate identifier
    validate_identifier(identifier.clone())?;
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
                            ("identifier", &identifier),
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

