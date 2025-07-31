use cosmwasm_std::{Response, StdError, StdResult};
use regex::Regex;   


// validate email format
pub fn validate_email(email: &str) -> StdResult<()> {
    let email_regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap(); 
    if email_regex.is_match(email) {
        Ok(())
    }else {
        Err(StdError::generic_err("Invalid email format"))
    }
}

// Validate username (alphanumeric, 3-32 characters)
pub fn validate_username(username: &str) -> StdResult<()> {
    let username_regex = Regex::new(r"^[a-zA-Z0-9]{3,32}$").unwrap();
    if username_regex.is_match(username) {
        Ok(())
    } else {
        Err(StdError::generic_err("Username must be 3-32 alphanumeric characters"))
    }
}

// validate identifier (email or username)
pub fn validate_identifier(identifier: &str) -> StdResult<()> {
    if identifier.contains('@') {
        validate_email(identifier)
    }else {
        validate_username(identifier)
    }
}

// standardize response creation 
pub fn create_response(action: &str, attributes: Vec<(&str, &str)>) -> Response {
    let mut response = Response::new().add_attribute("action", action);
    for (key, value) in attributes {
        response = response.add_attribute(key, value);
    }
    response
}

