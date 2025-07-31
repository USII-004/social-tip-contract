#[allow(clippy::module_inception)]
#[cfg(test)]
mod integration_tests {

    use cosmwasm_std::{
        testing::{message_info, mock_dependencies, mock_env, MockApi, MockQuerier, MockStorage},
        Addr, Coin, CosmosMsg, OwnedDeps, Response, StdError, StdResult, Uint128
    };
    use crate::contract::{execute, instantiate, query};
    use crate::msg::{
        ExecuteMsg, InstantiateMsg, QueryMsg, EscrowResponse, AccountResponse,
    };
    


    fn setup_contract(deps: &mut OwnedDeps<MockStorage, MockApi, MockQuerier>,
    ) -> (Response, String) {
        let env = mock_env();
        let info = message_info(&Addr::unchecked("creator"), &[]);
        let msg = InstantiateMsg {
            token_denom: "uxion".to_string(),
        };
        let res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
        let contract_addr = env.contract.address.to_string();
        (res, contract_addr)
    }

    #[test]
    fn test_full_workflow() -> StdResult<()> {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let sender = message_info(&Addr::unchecked("sender"), &[]);
        let recipient = message_info(&Addr::unchecked("recipient"), &[]);

        // Instantiate contract
        let (_res, _contract_addr) = setup_contract(&mut deps);

        // register sender  with email
        let register_msg = &ExecuteMsg::Register {
            identifier: "sender@mail.com".to_string(),
        };
        execute(deps.as_mut(), env.clone(), sender.clone(), register_msg)?;

        // Query account to verify registration
        let query_msg = QueryMsg::GetAccount { 
            identifier: "sender@mail.com".to_string(), 
        };
        let res = query(deps.as_ref(), env.clone(), query_msg)?;
        let account: AccountResponse = cosmwasm_std::from_json(&res)?;
        assert_eq!(account.address, Some(Addr::unchecked("sender")));

        // transfer to unregistered email (should go to escrow)
        let transfer_msg = &ExecuteMsg::Transfer { 
            identifier: "unregistered@mail.com".to_string(),
            amount: Coin {
                denom: "uxion".to_string(),
                amount: Uint128::from(10000000u128),
            },
        };
        let sender_with_funds = message_info(&Addr::unchecked("sender"), &[Coin {
            denom: "uxion".to_string(),
            amount: Uint128::from(10000000u128),
        }]);
        let res = execute(deps.as_mut(), env.clone(), sender_with_funds, transfer_msg)?;
        assert_eq!(res.attributes[0].value, "escrow");

        // Query escrow
        let query_escrow_msg = QueryMsg::GetEscrow { 
            identifier: "unregistered@mail.com".to_string(), 
        };
        let res = query(deps.as_ref(), env.clone(), query_escrow_msg)?;
        let escrow: EscrowResponse = cosmwasm_std::from_json(&res)?;
        assert_eq!(
            escrow.escrow.unwrap().amount,
            Coin {
                denom: "uxion".to_string(),
                amount: Uint128::from(10000000u128),
            }
        );

        // Register recipient with the unregistered email
        let register_recipient_msg = &ExecuteMsg::Register { 
            identifier: "unregistered@mail.com".to_string(), 
        };
        execute(deps.as_mut(), env.clone(), recipient.clone(), register_recipient_msg)?;

        //Claim escrowed token
        let claim_msg = &ExecuteMsg::Claim { 
            identifier: "unregistered@mail.com".to_string(), 
        };
        let res = execute(deps.as_mut(), env.clone(), recipient, claim_msg)?;
        assert_eq!(res.attributes[0].value, "claim");
        assert!(res.messages.iter().any(|msg| matches!(
            msg.msg,
            CosmosMsg::Bank(_),
        )));

        // Verify escrow is Clear
        let query_escrow_msg = QueryMsg::GetEscrow { 
            identifier: "unregistered@mail.com".to_string(), 
        };
        let res = query(deps.as_ref(), env.clone(), query_escrow_msg)?;
        let escrow: EscrowResponse = cosmwasm_std::from_json(&res)?;
        assert_eq!(escrow.escrow, None);

        Ok(())

    }

    #[test]
    fn test_invalid_email() -> StdResult<()> {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let sender = message_info(&Addr::unchecked("sender"), &[]);

        // Instantiate contract
        setup_contract(&mut deps);

        // Try registering with invalid email
        let register_msg = &ExecuteMsg::Register { 
            identifier: "invalid-email".to_string(), 
        };
        let res = execute(deps.as_mut(), env.clone(), sender.clone(), register_msg);
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err(),
            StdError::generic_err("Username must be 3-32 alphanumeric characters")
        );

        Ok(())  

    }

    #[test]
    fn test_duplicate_registration() -> StdResult<()> {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let sender = message_info(&Addr::unchecked("sender"), &[]);

        // Instantiate contract
        setup_contract(&mut deps);

        // Register email
        let register_msg = &ExecuteMsg::Register { 
            identifier: "user@mail.com".to_string(), 
        };
        execute(deps.as_mut(), env.clone(), sender.clone(), register_msg)?;

        // Try registering the same email again
        let res = execute(deps.as_mut(), env.clone(), sender, register_msg);
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err(),
            StdError::generic_err("Identifier already registered"),
        );

        Ok(())
    }

    #[test]
    fn test_transfer_wrong_denom() -> StdResult<()> {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let sender = message_info(&Addr::unchecked("sender"), &[Coin {
            denom: "wrongdenom".to_string(),
            amount: Uint128::from(10000000u128),
        }]);

        // Instantiate contract
        setup_contract(&mut deps);

        // Register sender
        let register_msg = &ExecuteMsg::Register { 
            identifier: "sender@mail.com".to_string(), 
        };
        execute(deps.as_mut(), env.clone(), message_info(&Addr::unchecked("sender"), &[]), register_msg)?;

        // Try transfering to wrong denom
        let transfer_msg = &ExecuteMsg::Transfer { 
            identifier: "recipient@mail.com".to_string(),
            amount: Coin {
                denom: "wrongdenom".to_string(),
                amount: Uint128::from(10000000u128),
            },
        };
        let res = execute(deps.as_mut(), env.clone(), sender, transfer_msg);
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err(),
            StdError::generic_err("Invalid token denomination"),
        );

        Ok(())

    }

}