use odra::{prelude::*, Address};

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::iter::Peekable;
use core::mem::discriminant;

use super::NameTokenMetadata;

#[derive(Debug)]
enum Token {
    LeftBrace,
    RightBrace,
    Colon,
    Comma,
    String(String),
    Number(u64),
    Null,
}

pub fn parse_name_token_metadata(json_str: &str) -> Result<NameTokenMetadata, String> {
    let tokens = tokenize(json_str)?;

    let mut tokens_iter = tokens.into_iter().peekable();

    fn expect_token(
        tokens: &mut Peekable<alloc::vec::IntoIter<Token>>,
        expected: &Token,
    ) -> Result<(), String> {
        if let Some(token) = tokens.next() {
            if discriminant(&token) != discriminant(expected) {
                return Err(format!("Expected {:?}, found {:?}", expected, token));
            } else {
                Ok(())
            }
        } else {
            Err(format!("Expected {:?}, found EOF", expected))
        }
    }

    fn expect_colon(tokens: &mut Peekable<alloc::vec::IntoIter<Token>>) -> Result<(), String> {
        expect_token(tokens, &Token::Colon)
    }

    expect_token(&mut tokens_iter, &Token::LeftBrace)?;

    let mut name: Option<String> = None;
    let mut expiration: Option<u64> = None;
    let mut resolver: Option<Option<String>> = None;

    loop {
        // Expect a String (key)
        let key = match tokens_iter.next() {
            Some(Token::String(s)) => s,
            Some(t) => return Err(format!("Expected string as key, found {:?}", t)),
            None => return Err("Expected string as key, found EOF".to_string()),
        };

        expect_colon(&mut tokens_iter)?;

        // Expect a value
        let value_token = match tokens_iter.next() {
            Some(token) => token,
            None => return Err("Expected value after colon, found EOF".to_string()),
        };

        match key.as_str() {
            "name" => match value_token {
                Token::String(s) => name = Some(s),
                _ => return Err(format!("Expected string value for 'name', found {:?}", value_token)),
            },
            "expiration" => match value_token {
                Token::Number(n) => expiration = Some(n),
                _ => return Err(format!("Expected number value for 'expiration', found {:?}", value_token)),
            },
            "resolver" => match value_token {
                Token::String(s) => resolver = Some(Some(s)),
                Token::Null => resolver = Some(None),
                _ => return Err(format!(
                    "Expected string or null value for 'resolver', found {:?}",
                    value_token
                )),
            },
            _ => {
                // Skip unknown keys and their corresponding values
            }
        }

        // After value, expect either Comma or RightBrace
        match tokens_iter.peek() {
            Some(Token::Comma) => {
                tokens_iter.next(); // consume comma
                continue;
            }
            Some(Token::RightBrace) => {
                tokens_iter.next(); // consume right brace
                break;
            }
            Some(t) => return Err(format!("Expected ',' or '}}', found {:?}", t)),
            None => return Err("Expected ',' or '}', found EOF".to_string()),
        }
    }

    // Check that required fields are present
    if name.is_none() || expiration.is_none() || resolver.is_none() {
        return Err("Missing required fields".to_string());
    }

    let resolver_address = if let Some(resolver) = resolver.unwrap() {
        match Address::from_str(&resolver) {
            Ok(address) => Some(address),
            Err(_) => return Err("Invalid address".to_string()),
        }
    } else {
        None
    };

    Ok(NameTokenMetadata {
        name: name.unwrap(),
        expiration: expiration.unwrap(),
        resolver: resolver_address,
    })
}

fn tokenize(s: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut chars = s.chars().peekable();
    while let Some(&c) = chars.peek() {
        match c {
            ' ' | '\n' | '\r' | '\t' => {
                // Skip whitespace
                chars.next();
            }
            '{' => {
                tokens.push(Token::LeftBrace);
                chars.next();
            }
            '}' => {
                tokens.push(Token::RightBrace);
                chars.next();
            }
            ':' => {
                tokens.push(Token::Colon);
                chars.next();
            }
            ',' => {
                tokens.push(Token::Comma);
                chars.next();
            }
            '"' => {
                // String
                chars.next(); // consume the opening quote
                let mut string = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '"' {
                        chars.next(); // consume the closing quote
                        break;
                    } else if c == '\\' {
                        // Handle escaped characters
                        chars.next(); // consume the backslash
                        if let Some(&escaped_char) = chars.peek() {
                            // For simplicity, we can just add the escaped character
                            string.push(escaped_char);
                            chars.next();
                        } else {
                            return Err("Invalid escape sequence in string".to_string());
                        }
                    } else {
                        string.push(c);
                        chars.next();
                    }
                }
                tokens.push(Token::String(string));
            }
            '0'..='9' => {
                // Number
                let mut num_str = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_digit() {
                        num_str.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                let num = match num_str.parse::<u64>() {
                    Ok(n) => n,
                    Err(_) => return Err(format!("Invalid number: {}", num_str)),
                };
                tokens.push(Token::Number(num));
            }
            'n' => {
                // null
                let mut null_str = String::new();
                for _ in 0..4 {
                    if let Some(&c) = chars.peek() {
                        null_str.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if null_str == "null" {
                    tokens.push(Token::Null);
                } else {
                    return Err(format!("Invalid token: {}", null_str));
                }
            }
            _ => {
                return Err(format!("Unexpected character: {}", c));
            }
        }
    }
    Ok(tokens)
}
