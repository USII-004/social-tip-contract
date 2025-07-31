use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Coin};
use cw_storage_plus::{Item, Map};

// store the token denomination
pub const TOKEN_DENOM: Item<String> = Item::new("token_denom");

//Map username/email to XION address
pub  const ACCOUNTS: Map<String, Addr> = Map::new("accounts");

// store escrowed token for unregistered emails
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Escrow {
    pub sender: Addr,
    pub amount: Coin,
}

pub const ESCROWS: Map<String, Escrow> = Map::new("escrows");
