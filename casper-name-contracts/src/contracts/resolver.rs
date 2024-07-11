use odra::{args::Maybe, prelude::*, Address, External, Mapping, SubModule, UnwrapOrRevert};
use odra_modules::access::{AccessControl, Role, DEFAULT_ADMIN_ROLE};

use super::{name_token::NameTokenContractRef, utils};

#[odra::external_contract]
pub trait Resolver {
    fn init(&mut self, name_token: Address);
    fn set_name_token(&mut self, name_token: Address);
    fn set_resolution(&mut self, full_domain: String, address: Option<Address>);
    fn resolve(&self, full_domain: String) -> Option<Address>;
    fn cleanup(&mut self, token_name: String);
}

type TokenHash = String;
type Domain = String;
type Nonce = u32;

#[odra::module]
pub struct DefaultResolver {
    access_control: SubModule<AccessControl>,
    name_token: External<NameTokenContractRef>,
    nonces: Mapping<TokenHash, Nonce>,
    resolutions: Mapping<(TokenHash, Domain, Nonce), Option<Address>>,
}

#[odra::module]
impl DefaultResolver {
    delegate! {
        to self.access_control {
            fn has_role(&self, role: &Role, address: &Address) -> bool;
            fn grant_role(&mut self, role: &Role, address: &Address);
            fn revoke_role(&mut self, role: &Role, address: &Address);
        }
    }

    pub fn init(&mut self, name_token: Address) {
        self.set_name_token(name_token);

        let admin = self.env().caller();
        self.access_control
            .unchecked_grant_role(&DEFAULT_ADMIN_ROLE, &admin);
    }

    pub fn set_name_token(&mut self, name_token: Address) {
        self.name_token.set(name_token);
    }

    pub fn set_resolution(&mut self, full_domain: String, address: Option<Address>) {
        let token_hash = self.calculate_token_hash(&full_domain).unwrap_or_revert(self);
        let caller = self.env().caller();
 
        if !self.name_token.is_token_valid(&token_hash) {
            self.env().revert(ResolverError::ResolutionSetWithInvalidToken);
        }

        if self.name_token.owner_of(Maybe::None, Maybe::Some(token_hash.clone())) != caller {
            self.env().revert(ResolverError::ResolutionSetByInvalidOwner);
        }

        let nonce = self.nonce(&token_hash);
        self.resolutions
            .set(&(token_hash, full_domain, nonce), address);
    }

    pub fn resolve(&self, full_domain: String) -> Option<Address> {
        let token_hash = self.calculate_token_hash(&full_domain)?;
        let nonce = self.nonce(&token_hash);

        self.resolutions
            .get(&(token_hash, full_domain, nonce))
            .flatten()
    }

    pub fn cleanup(&mut self, token_name: String) {
        let hash = self.env().hash(token_name);
        let token_hash = utils::to_utf8_string(&hash).unwrap_or_revert(self);
        self.nonces.add(&token_hash, 1);
    }

    #[inline]
    fn nonce(&self, token_hash: &TokenHash) -> Nonce {
        self.nonces.get_or_default(token_hash)
    }

    #[inline]
    fn calculate_token_hash(&self, full_domain: &str) -> Option<TokenHash> {
        let token_name = utils::extract_token_name(&full_domain).unwrap_or_revert(self);
        let hash = self.env().hash(token_name);
        Some(utils::to_utf8_string(&hash).unwrap_or_revert(self))
    }
}

#[odra::odra_error]
pub enum ResolverError {
    ResolutionSetWithInvalidToken = 1500,
    ResolutionSetByInvalidOwner = 1501,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_context::TestContext;

    #[test]
    fn test_default_resolver() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice, bob) = (ctx.admin, ctx.alice, ctx.bob);
        let token_name = "odra";

        ctx.with_name_registered(admin, alice, token_name);

        let main_domain = "odra.cspr".to_string();
        let subdomain = "docs.odra.cspr".to_string();

        ctx.set_caller(alice);
        ctx.default_resolver.set_resolution(main_domain.clone(), Some(alice));
        ctx.default_resolver.set_resolution(subdomain.clone(), Some(bob));

        assert_eq!(ctx.default_resolver.resolve(main_domain.clone()), Some(alice));
        assert_eq!(ctx.default_resolver.resolve(subdomain.clone()), Some(bob));

        ctx.default_resolver.cleanup(token_name.to_string());

        assert_eq!(ctx.default_resolver.resolve(main_domain), None);
        assert_eq!(ctx.default_resolver.resolve(subdomain), None);
    }
}
