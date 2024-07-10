use odra::{prelude::*, Address, External, Var};

use super::name_token::NameTokenContractRef;

#[odra::external_contract]
pub trait Resolver {
    fn init(&mut self, name_token: Address);
    fn set_name_token(&mut self, name_token: Address);
    fn set_resolution(&mut self, full_domain: String, address: Option<Address>);
    fn resolve(&self, full_domain: String) -> Option<Address>;
    fn cleanup(&mut self, token_id: String);
}

#[odra::module]
pub struct MockResolver {
    name_token: External<NameTokenContractRef>,
    resolutions: Var<BTreeMap<String, Option<Address>>>,
}

#[odra::module]
impl MockResolver {
    pub fn init(&mut self, name_token: Address) {
        self.name_token.set(name_token);
    }

    pub fn set_name_token(&mut self, name_token: Address) {
        self.name_token.set(name_token);
    }

    pub fn set_resolution(&mut self, full_domain: String, address: Option<Address>) {
        let mut resolutions = self.resolutions.get_or_default();
        resolutions.insert(full_domain, address);
        self.resolutions.set(resolutions);
    }

    pub fn resolve(&self, full_domain: String) -> Option<Address> {
        let resolutions = self.resolutions.get()?;
        resolutions.get(&full_domain).copied().flatten()
    }

    pub fn cleanup(&mut self, token_id: String) {
        self.resolutions.set(BTreeMap::new());
    }
}
