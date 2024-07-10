use odra::{prelude::*, Address, External, Mapping, Var};

use super::name_token::NameTokenContractRef;

#[odra::external_contract]
pub trait Resolver {
    fn init(&mut self, name_token: Address);
    fn set_name_token(&mut self, name_token: Address);
    fn set_resolution(&mut self, full_domain: String, address: Option<Address>);
    fn resolve(&self, full_domain: String) -> Option<Address>;
    fn cleanup(&mut self, token_id: String);
}

pub type TokenHash = String;
pub type Subdomain = String;
pub type Nonce = u32;

#[odra::module]
pub struct DefaultResolver {
    nonces: Mapping<TokenHash, Nonce>,
    resolutions: Mapping<(TokenHash,Subdomain, Nonce), Address>
}

#[odra::module]
impl DefaultResolver {
    pub fn init(&mut self) {}

    pub fn set_resolution(&mut self, token_hash: TokenHash, subdomain: Subdomain, address: Address) {
        let nonce = self.nonce(&token_hash);
        self.resolutions.set(&(token_hash, subdomain, nonce), address);
    }

    pub fn resolve(&self, token_hash: TokenHash, subdomain: Subdomain) -> Option<Address> {
        let nonce = self.nonce(&token_hash);
        self.resolutions.get(&(token_hash, subdomain, nonce))
    }

    fn nonce(&self, token_hash: &TokenHash) -> Nonce {
        self.nonces.get_or_default(token_hash)
    }

    pub fn cleanup(&mut self, token_hash: TokenHash) {
        self.nonces.add(&token_hash, 1);
    }
}

#[cfg(test)]
mod tests {
    use odra::host::{Deployer, NoArgs};

    use super::*;
    
    #[test]
    fn test_default_resolver() {
        let env = odra_test::env();
        let mut resolver = DefaultResolverHostRef::deploy(&env, NoArgs);

        let token_hash = "token_hash".to_string();
        let subdomain = "subdomain".to_string();
        let address = env.get_account(4);

        resolver.set_resolution(token_hash.clone(), subdomain.clone(), address);
        assert_eq!(resolver.resolve(token_hash.clone(), subdomain.clone()), Some(address));

        resolver.cleanup(token_hash.clone());

        assert_eq!(resolver.resolve(token_hash.clone(), subdomain.clone()), None);
    }        
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

    pub fn cleanup(&mut self, #[allow(unused_variables)] token_id: String) {
        self.resolutions.set(BTreeMap::new());
    }
}
