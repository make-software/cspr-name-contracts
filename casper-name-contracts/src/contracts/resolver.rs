use core::ops::Deref;

use odra::{args::Maybe, prelude::*, Address, External, Mapping, SubModule, UnwrapOrRevert};
use odra_modules::access::{AccessControl, Role, DEFAULT_ADMIN_ROLE};

use super::{name_token::NameTokenContractRef, utils};

#[odra::external_contract]
pub trait Resolver {
    fn init(&mut self, name_token: Address);
    fn set_name_token(&mut self, name_token: Address);
    fn set_resolution(&mut self, full_domain: Domain, address: Option<Address>);
    fn resolve(&self, full_domain: Domain) -> Option<Address>;
    fn cleanup(&mut self, token_hash: TokenHash);
}

type Nonce = u32;
type Domain = String;
type TokenHash = String;

/// Event emitted when a resolution is changed.
#[odra::event]
pub struct ResolutionChanged {
    full_domain: String,
    address: Option<Address>,
}

/// Event emitted when a resolution is cleared.
#[odra::event]
pub struct ResolutionCleared {
    token_hash: String,
}

/// Default Resolver smart contract. It handles the resolution of domain names to addresses.
#[odra::module(events = [ResolutionChanged, ResolutionCleared])]
pub struct DefaultResolver {
    access_control: SubModule<AccessControl>,
    name_token: External<NameTokenContractRef>,
    nonces: Mapping<String, Nonce>,
    resolutions: Mapping<(String, String, Nonce), Option<Address>>,
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

    /// Initializes the default resolver with the name token contract address.
    /// The caller is granted the admin role.
    pub fn init(&mut self, name_token: Address) {
        self.name_token.set(name_token);

        let admin = self.env().caller();
        self.access_control
            .unchecked_grant_role(&DEFAULT_ADMIN_ROLE, &admin);
    }

    /// Admin only. Sets the name token contract address.
    pub fn set_name_token(&mut self, name_token: Address) {
        if !self.has_role(&DEFAULT_ADMIN_ROLE, &self.env().caller()) {
            self.env()
                .revert(ResolverError::UnauthorizedTokenAddressUpdate);
        }
        self.name_token.set(name_token);
    }

    /// Token owner only. Sets the resolution for a domain to an address.
    pub fn set_resolution(&mut self, full_domain: Domain, address: Option<Address>) {
        let env = self.env();
        let token_hash = self
            .calculate_token_hash(&full_domain)
            .unwrap_or_revert_with(self, ResolverError::InvalidDomain);
        let caller = env.caller();

        if !self.name_token.is_token_valid(&token_hash) {
            env.revert(ResolverError::ResolutionSetWithInvalidToken);
        }

        if self.owner_of(&token_hash) != caller {
            env.revert(ResolverError::ResolutionSetByInvalidOwner);
        }

        let nonce = self.nonce(&token_hash);
        self.resolutions
            .set(&(token_hash, full_domain.clone(), nonce), address);

        env.emit_event(ResolutionChanged {
            full_domain,
            address,
        });
    }

    /// Resolves a domain to an address.
    pub fn resolve(&self, full_domain: Domain) -> Option<Address> {
        let token_hash = self.calculate_token_hash(&full_domain)?;
        let nonce = self.nonce(&token_hash);

        self.resolutions
            .get(&(token_hash, full_domain, nonce))
            .flatten()
    }

    /// Cleanup the resolutions for a token. Only the token owner or the admin can do this.
    pub fn cleanup(&mut self, token_hash: TokenHash) {
        let env = self.env();
        let caller = env.caller();

        if !self.has_role(&DEFAULT_ADMIN_ROLE, &caller) && self.owner_of(&token_hash) != caller {
            self.env().revert(ResolverError::UnauthorizedCleanup);
        }
        self.nonces.add(&token_hash, 1);

        env.emit_event(ResolutionCleared {
            token_hash: token_hash.deref().to_owned(),
        });
    }

    #[inline]
    fn calculate_token_hash(&self, full_domain: &str) -> Option<TokenHash> {
        let token_name = utils::extract_token_name(&full_domain)?;
        let hash = self.env().hash(token_name);
        Some(utils::to_utf8_string(&hash).unwrap_or_revert(self))
    }

    #[inline]
    fn nonce(&self, token_hash: &TokenHash) -> Nonce {
        self.nonces.get_or_default(token_hash)
    }

    #[inline]
    fn owner_of(&self, token_hash: &TokenHash) -> Address {
        self.name_token
            .owner_of(Maybe::None, Maybe::Some(token_hash.to_owned()))
    }
}

#[odra::odra_error]
pub enum ResolverError {
    ResolutionSetWithInvalidToken = 1401,
    ResolutionSetByInvalidOwner = 1402,
    UnauthorizedCleanup = 1403,
    UnauthorizedTokenAddressUpdate = 1404,
    InvalidDomain = 1405,
}

#[cfg(test)]
mod tests {
    use odra::OdraResult;

    use super::*;
    use crate::test_context::{blake2b, TestContext, TOKEN_EXPIRATION};

    const TOKEN_NAME: &str = "odra";
    const NON_EXISTENT_TOKEN_DOMAIN: &str = "odra2.cspr";
    const NON_CSPR_DOMAIN: &str = "odra.com";
    const MAIN_DOMAIN: &str = "odra.cspr";
    const SUBDOMAIN: &str = "docs.odra.cspr";

    #[test]
    fn deployer_is_admin() {
        // Given the contract is deployed
        let (ctx, admin, _, _) = setup();
        // Then the deployer is the admin
        assert!(ctx.default_resolver.has_role(&DEFAULT_ADMIN_ROLE, &admin));
    }

    #[test]
    fn only_admin_can_set_name_token() {
        let (mut ctx, admin, alice, _) = setup();

        // When alice tries to set the name token
        ctx.set_caller(alice);
        // Then the operation fails
        assert!(ctx.default_resolver.try_set_name_token(alice).is_err());

        // When the admin sets the name token
        ctx.set_caller(admin);
        // Then the operation succeeds
        assert!(ctx.default_resolver.try_set_name_token(alice).is_ok());
    }

    #[test]
    fn anyone_can_resolve() {
        let (mut ctx, _, alice, bob) = setup();

        // When alice sets the resolution for the main domain
        set_resolution_with_caller(&mut ctx, MAIN_DOMAIN, alice, alice);
        // Then alice can resolve the main domain
        assert_eq!(
            resolve_with_caller(&mut ctx, MAIN_DOMAIN, alice),
            Some(alice)
        );
        // And bob can also resolve the main domain
        assert_eq!(resolve_with_caller(&mut ctx, MAIN_DOMAIN, bob), Some(alice));
    }

    #[test]
    fn only_owner_can_set_resolution() {
        let (mut ctx, admin, alice, bob) = setup();

        // When alice sets the resolution for the main domain
        ctx.set_caller(alice);
        // Then the operation succeeds
        assert!(try_set_resolution(&mut ctx, MAIN_DOMAIN, alice).is_ok());

        // When admin sets the resolution for the main domain
        ctx.set_caller(admin);
        // Then the operation fails
        assert_eq!(
            try_set_resolution(&mut ctx, MAIN_DOMAIN, admin).err(),
            Some(ResolverError::ResolutionSetByInvalidOwner.into())
        );
        // When bob sets the resolution for the main domain
        ctx.set_caller(bob);
        // Then the operation fails
        assert_eq!(
            try_set_resolution(&mut ctx, MAIN_DOMAIN, bob).err(),
            Some(ResolverError::ResolutionSetByInvalidOwner.into())
        );
    }

    #[test]
    fn cannot_set_resolution_with_non_existent_token() {
        let (mut ctx, _, alice, _) = setup();

        // When alice tries to set the resolution for a non-existent token
        ctx.set_caller(alice);
        let result = try_set_resolution(&mut ctx, NON_EXISTENT_TOKEN_DOMAIN, alice);
        // Then the operation fails
        assert_eq!(
            result,
            Err(ResolverError::ResolutionSetWithInvalidToken.into())
        );
    }

    #[test]
    fn cannot_set_resolution_with_expired_token() {
        // Given the token has expired
        let (mut ctx, _, alice, _) = setup();
        ctx.advance_block_time(TOKEN_EXPIRATION + 1);

        // When alice tries to set the resolution
        ctx.set_caller(alice);
        let result = try_set_resolution(&mut ctx, SUBDOMAIN, alice);
        // Then the operation fails
        assert_eq!(
            result,
            Err(ResolverError::ResolutionSetWithInvalidToken.into())
        );
    }

    #[test]
    fn cannot_set_resolution_with_non_cspr_domain() {
        let (mut ctx, _, alice, _) = setup();
        // When alice tries to set the resolution for a non-cspr domain
        ctx.set_caller(alice);
        let result = try_set_resolution(&mut ctx, NON_CSPR_DOMAIN, alice);
        // Then the operation fails
        assert_eq!(result, Err(ResolverError::InvalidDomain.into()));
    }

    #[test]
    fn cannot_set_resolution_with_burned_token() {
        // Given the token has been burned
        let (mut ctx, admin, alice, _) = setup();
        ctx.set_caller(admin);
        ctx.token
            .set_variables(Maybe::Some(true), Maybe::Some(vec![admin]), Maybe::None);
        ctx.token
            .burn(Maybe::None, Maybe::Some(blake2b(TOKEN_NAME)));
        // When alice tries to set the resolution
        ctx.set_caller(alice);
        let result = try_set_resolution(&mut ctx, SUBDOMAIN, alice);
        // Then the operation fails
        assert_eq!(
            result,
            Err(ResolverError::ResolutionSetWithInvalidToken.into())
        );
    }

    #[test]
    fn cleanup_erases_subdomains() {
        let (mut ctx, _, alice, bob) = setup();

        // When alice sets the resolution for the main domain and a subdomain
        ctx.set_caller(alice);
        set_resolution(&mut ctx, MAIN_DOMAIN, alice);
        set_resolution(&mut ctx, SUBDOMAIN, bob);

        // Then both resolutions are set
        assert_eq!(resolve(&ctx, MAIN_DOMAIN), Some(alice));
        assert_eq!(resolve(&ctx, SUBDOMAIN), Some(bob));

        // When alice cleans up the token's resolutions
        cleanup(&mut ctx, TOKEN_NAME);

        // Then both resolutions are erased
        assert_eq!(resolve(&ctx, MAIN_DOMAIN), None);
        assert_eq!(resolve(&ctx, SUBDOMAIN), None);
    }

    #[test]
    fn admin_can_cleanup_any_token() {
        let (mut ctx, admin, alice, bob) = setup();

        // When alice sets the resolution for the main domain and a subdomain
        ctx.set_caller(alice);
        set_resolution(&mut ctx, MAIN_DOMAIN, alice);
        set_resolution(&mut ctx, SUBDOMAIN, bob);

        // Then both resolutions are set
        assert_eq!(resolve(&ctx, MAIN_DOMAIN), Some(alice));
        assert_eq!(resolve(&ctx, SUBDOMAIN), Some(bob));

        // When the admin cleans up alice's token's resolutions
        cleanup_with_caller(&mut ctx, TOKEN_NAME, admin);

        // Then both resolutions are erased
        assert_eq!(resolve(&ctx, MAIN_DOMAIN), None);
        assert_eq!(resolve(&ctx, SUBDOMAIN), None);
    }

    #[test]
    fn only_owner_or_admin_can_cleanup() {
        let (mut ctx, _, alice, bob) = setup();

        // When alice sets the resolution for the main domain and a subdomain
        ctx.set_caller(alice);
        set_resolution(&mut ctx, MAIN_DOMAIN, alice);
        set_resolution(&mut ctx, SUBDOMAIN, bob);

        // Then both resolutions are set
        assert_eq!(resolve(&ctx, MAIN_DOMAIN), Some(alice));
        assert_eq!(resolve(&ctx, SUBDOMAIN), Some(bob));

        // When bob tries to clean up alice's token's resolutions
        ctx.set_caller(bob);
        let result = try_cleanup(&mut ctx, TOKEN_NAME);
        // Then the operation fails
        assert_eq!(result, Err(ResolverError::UnauthorizedCleanup.into()));
    }

    fn setup() -> (TestContext, Address, Address, Address) {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice, bob) = (ctx.admin, ctx.alice, ctx.bob);

        ctx.with_name_registered(admin, alice, TOKEN_NAME);
        (ctx, admin, alice, bob)
    }

    fn set_resolution(ctx: &mut TestContext, domain: &str, address: Address) {
        ctx.default_resolver
            .set_resolution(domain.to_string(), Some(address));
    }

    fn try_set_resolution(ctx: &mut TestContext, domain: &str, address: Address) -> OdraResult<()> {
        ctx.default_resolver
            .try_set_resolution(domain.to_string(), Some(address))
    }

    fn set_resolution_with_caller(
        ctx: &mut TestContext,
        domain: &str,
        address: Address,
        caller: Address,
    ) {
        ctx.set_caller(caller);
        set_resolution(ctx, domain, address);
    }

    fn resolve(ctx: &TestContext, domain: &str) -> Option<Address> {
        ctx.default_resolver.resolve(domain.to_string())
    }

    fn resolve_with_caller(
        ctx: &mut TestContext,
        domain: &str,
        caller: Address,
    ) -> Option<Address> {
        ctx.set_caller(caller);
        resolve(ctx, domain)
    }

    fn cleanup(ctx: &mut TestContext, token_name: &str) {
        try_cleanup(ctx, token_name).unwrap();
    }

    fn cleanup_with_caller(ctx: &mut TestContext, token_hash: &str, caller: Address) {
        ctx.set_caller(caller);
        cleanup(ctx, token_hash);
    }

    fn try_cleanup(ctx: &mut TestContext, token_name: &str) -> OdraResult<()> {
        ctx.default_resolver.try_cleanup(blake2b(token_name))
    }
}
