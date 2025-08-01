#[allow(clippy::module_inception)]
#[cfg(test)]
mod integration_tests {

    use cosmwasm_std::{
        testing::{message_info, mock_dependencies, mock_env, MockApi, MockQuerier, MockStorage}, Addr, BankMsg, Coin, CosmosMsg, OwnedDeps, Response, StdError, StdResult, Uint128
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
            platform_wallet: "platform_wallet".to_string(),
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

        let instantiate_msg = InstantiateMsg {
            token_denom: "uxion".to_string(),
            platform_wallet: "platform_wallet".to_string(),
        };

        // Instantiate contract
        let _ = instantiate(deps.as_mut(), env.clone(), sender.clone(), instantiate_msg);

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
        let full_amount = Uint128::from(10_000_000u128);
        let platform_fee = full_amount.multiply_ratio(1u128, 100u128);
        let expected_escrowed = full_amount.checked_sub(platform_fee).unwrap();

        let transfer_msg = &ExecuteMsg::Transfer { 
            identifier: "unregistered@mail.com".to_string(),
            amount: Coin {
                denom: "uxion".to_string(),
                amount: full_amount,
            },
        };
        let sender_with_balance = message_info(&Addr::unchecked("sender"), &[Coin {
            denom: "uxion".to_string(),
            amount: full_amount,
        }]);
        let res = execute(deps.as_mut(), env.clone(), sender_with_balance, transfer_msg)?;
        
        // check that escrow was created
        assert_eq!(res.attributes[0].value, "escrow");

        // check that platform fee was sent in a BankMsg
        let fee_msg = res.messages.iter().find_map(|msg| match &msg.msg{
            CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                if to_address == "platform_wallet" {
                    Some(amount.clone())
                }else {
                    None
                }
            }
            _ => None,
        });

        assert_eq!(
            fee_msg.unwrap(),
            vec![Coin {
                denom: "uxion".to_string(),
                amount: platform_fee,
            }]
        );

        // Query escrow
        let query_escrow_msg = QueryMsg::GetEscrow { 
            identifier: "unregistered@mail.com".to_string(), 
        };
        let res = query(deps.as_ref(), env.clone(), query_escrow_msg)?;
        let escrow: EscrowResponse = cosmwasm_std::from_json(&res)?;
        assert!(
            escrow.escrow.is_some(),
            "Expected escrow for 'unregistered@mail.com' but found none"
        );
        assert_eq!(
            escrow.escrow.unwrap().amount,
            Coin {
                denom: "uxion".to_string(),
                amount: expected_escrowed,
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

        // verify the claimed amount matches escrow
        let transfer_back = res.messages.iter().find_map(|msg| match &msg.msg {
            CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                if to_address == "recipient" {
                    Some(amount.clone())
                }else {
                    None
                }
            }
            _ => None
        });
        assert_eq!(
            transfer_back.unwrap(),
            vec![Coin {
                denom: "uxion".to_string(),
                amount: expected_escrowed,
            }]
        );


        // Verify escrow is Clear
        let escrow_check = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::GetEscrow {
                identifier: "recipient@mail.com".to_string(),
            },
        )?;
        let escrow: EscrowResponse = cosmwasm_std::from_json(&escrow_check)?;
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