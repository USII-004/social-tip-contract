use cosmwasm_std::{Addr, Coin};
use schemars::JsonSchema;
use serde::{Serialize, Deserialize};
use crate::state::Escrow;
use cosmwasm_schema::QueryResponses;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub token_denom: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Register { identifier: String }, // register username/email
    Transfer { identifier: String, amount: Coin }, // Transfer token
    Claim { identifier: String }, // Claim escrowed tokens
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, QueryResponses)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    #[returns(BalanceResponse)]
    GetBalance { address: String },
    #[returns(EscrowResponse)]
    GetEscrow { identifier: String },
    #[returns(AccountResponse)]
    GetAccount { identifier: String },
}

// We define a custom struct for each query response

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BalanceResponse {
    pub balance: Coin,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct EscrowResponse {
    pub escrow: Option<Escrow>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AccountResponse {
    pub address: Option<Addr>,
}
