use odra::{prelude::*, Address};

#[odra::external_contract]
pub trait Resolver {
    fn init(&mut self, name_token: Address);
    fn set_name_token(&mut self, name_token: Address);
    fn set_resolution(&mut self, full_domain: String, address: Option<Address>);
    fn resolve(&self, full_domain: String) -> Option<Address>;
    fn cleanup(&mut self, token_id: String);
}
