use crate::contracts::{
    name_token::NameTokenContractRef, resolver::ResolverContractRef, token_id::ToTokenId, utils,
};
use odra::{prelude::*, ContractRef};

#[odra::odra_error]
enum Error {
    InvalidTokenName = 1500,
    ResolutionForPrimaryNameNotFound = 1501,
    InvalidResolutionAddress = 1502,
}

/// Reverse Resolver contract. It resolves primary names to addresses.
#[odra::module(events = [PrimaryNameChanged], errors = Error)]
pub struct ReverseResolver {
    primary_names: Mapping<Address, Option<String>>,
    name_token: External<NameTokenContractRef>,
}

#[odra::module]
impl ReverseResolver {
    pub fn init(&mut self, name_token: Address) {
        if !name_token.is_contract() {
            self.revert(Error::InvalidTokenName);
        }
        self.name_token.set(name_token);
    }

    /// Sets the primary preferred reverse resolution address for the caller.
    pub fn set_primary_name(&mut self, primary_name: String) {
        // Load currently set primary name.
        let caller = self.env().caller();
        let current_primary_name = self.get_primary_name(&caller);

        match self.existing_resolution(&primary_name) {
            Some(address) => {
                if address != caller {
                    // If the resolution exists but is not for the caller, revert.
                    self.revert(Error::InvalidResolutionAddress);
                }
            }
            None => {
                self.revert(Error::ResolutionForPrimaryNameNotFound);
            }
        };

        // Don't set the same primary name again.
        if current_primary_name.as_ref() != Some(&primary_name) {
            self.primary_names.set(&caller, Some(primary_name.clone()));
        }

        // Emit event.
        self.env().emit_event(PrimaryNameChanged {
            address: caller,
            old_primary_name: current_primary_name,
            new_primary_name: Some(primary_name),
        });
    }

    /// Returns the primary name for the address.
    pub fn get_primary_name(&self, address: &Address) -> Option<String> {
        let primary_name = self.primary_names.get(address).flatten()?;
        // Check if the primary name resolves to the given address.
        // If a resolver was cleaned up, it might not resolve anymore.
        let resolved_address = self.existing_resolution(&primary_name)?;
        if resolved_address == *address {
            Some(primary_name)
        } else {
            None
        }
    }

    fn existing_resolution(&self, name: &str) -> Option<Address> {
        let token_name = utils::extract_token_name(name)?;
        let token_id = self.token_id(token_name);

        // Check if the token ID is valid and if it has a resolver set. If not, return None.
        if !self.name_token.is_token_valid(token_id) {
            return None;
        }

        let resolver_address = self.name_token.resolver(token_id)?;
        ResolverContractRef::new(self.env(), resolver_address).resolve(name.to_owned())
    }
}

/// Event emitted when the primary name of an address changes.
#[odra::event]
pub struct PrimaryNameChanged {
    pub address: Address,
    pub old_primary_name: Option<String>,
    pub new_primary_name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_context::{self, TestContext};
    use odra::host::Deployer;

    const TOKEN_TEST: &str = "test";
    const TOKEN_TEST2: &str = "test2";
    const DOMAIN_TEST: &str = "test.cspr";
    const DOMAIN_TEST2: &str = "test2.cspr";

    #[test]
    fn test_set_primary_name() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, user) = (ctx.admin, ctx.alice);

        let mut reverse_resolver = ReverseResolver::deploy(
            &ctx.env,
            ReverseResolverInitArgs {
                name_token: ctx.token.address(),
            },
        );

        ctx.with_name_registered(admin, user, TOKEN_TEST);
        ctx.with_name_registered(admin, user, TOKEN_TEST2);

        ctx.set_caller(user);
        ctx.default_resolver
            .set_resolution(DOMAIN_TEST.to_string(), Some(user));
        ctx.default_resolver
            .set_resolution(DOMAIN_TEST2.to_string(), Some(user));

        // It should have no primary name.
        assert_eq!(reverse_resolver.get_primary_name(&user), None);

        // Set primary name.
        reverse_resolver.set_primary_name(DOMAIN_TEST.to_string());

        // It should have the primary name.
        assert_eq!(
            reverse_resolver.get_primary_name(&user),
            Some(DOMAIN_TEST.to_string())
        );

        // Set new primary name.
        reverse_resolver.set_primary_name(DOMAIN_TEST2.to_string());

        // It should have the new primary name.
        assert_eq!(
            reverse_resolver.get_primary_name(&user),
            Some(DOMAIN_TEST2.to_string())
        );
    }

    #[test]
    fn test_set_same_primary_name() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, user) = (ctx.admin, ctx.alice);

        let mut reverse_resolver = ReverseResolver::deploy(
            &ctx.env,
            ReverseResolverInitArgs {
                name_token: ctx.token.address(),
            },
        );

        ctx.with_name_registered(admin, user, TOKEN_TEST);
        ctx.with_name_registered(admin, user, TOKEN_TEST2);

        ctx.set_caller(user);
        ctx.default_resolver
            .set_resolution(DOMAIN_TEST.to_string(), Some(user));
        ctx.default_resolver
            .set_resolution(DOMAIN_TEST2.to_string(), Some(user));

        // Set primary name.
        ctx.set_caller(user);
        reverse_resolver.set_primary_name(DOMAIN_TEST.to_string());

        // Set a different primary name.
        ctx.set_caller(user);
        reverse_resolver.set_primary_name(DOMAIN_TEST2.to_string());

        // Set the same primary name again.
        ctx.set_caller(user);
        reverse_resolver.set_primary_name(DOMAIN_TEST2.to_string());

        // The contract should not emit an event for the same primary name.
        assert_eq!(ctx.events_count(&reverse_resolver), 3);
    }

    #[test]
    fn test_set_name_assigned_to_someone_else() {
        let (mut ctx, mut reverse_resolver) = setup();
        let (admin, user) = (ctx.admin, ctx.alice);

        ctx.with_name_registered(admin, user, TOKEN_TEST);

        ctx.set_caller(user);
        ctx.default_resolver
            .set_resolution(DOMAIN_TEST.to_string(), Some(admin));

        // Set primary name.
        let result = reverse_resolver.try_set_primary_name(DOMAIN_TEST.to_string());
        assert_eq!(result.unwrap_err(), Error::InvalidResolutionAddress.into());
    }

    #[test]
    fn test_set_not_existing_name() {
        let (mut ctx, mut reverse_resolver) = setup();
        let (admin, user) = (ctx.admin, ctx.alice);

        ctx.with_name_registered(admin, user, TOKEN_TEST);

        // Set primary name.
        ctx.set_caller(user);
        let result = reverse_resolver.try_set_primary_name(DOMAIN_TEST.to_string());
        assert_eq!(
            result.unwrap_err(),
            Error::ResolutionForPrimaryNameNotFound.into()
        );
    }

    #[test]
    fn test_get_primary_name() {
        let (mut ctx, mut reverse_resolver) = setup();
        let (admin, user) = (ctx.admin, ctx.alice);
        ctx.with_name_registered(admin, user, TOKEN_TEST);

        ctx.set_caller(user);
        ctx.default_resolver
            .set_resolution(DOMAIN_TEST.to_string(), Some(user));

        assert_eq!(reverse_resolver.get_primary_name(&user), None);

        reverse_resolver.set_primary_name(DOMAIN_TEST.to_string());
        assert_eq!(
            reverse_resolver.get_primary_name(&user),
            Some(DOMAIN_TEST.to_string())
        );
    }

    #[test]
    fn test_get_primary_name_invalidated_by_resolver() {
        let (mut ctx, mut reverse_resolver) = setup();
        let (admin, user) = (ctx.admin, ctx.alice);
        ctx.with_name_registered(admin, user, TOKEN_TEST);

        ctx.set_caller(user);
        ctx.default_resolver
            .set_resolution(DOMAIN_TEST.to_string(), Some(user));

        reverse_resolver.set_primary_name(DOMAIN_TEST.to_string());
        assert_eq!(
            reverse_resolver.get_primary_name(&user),
            Some(DOMAIN_TEST.to_string())
        );

        // Simulate a resolver cleanup by removing the resolution.
        ctx.default_resolver
            .invalidate_resolutions(test_context::generate_token_id(TOKEN_TEST));

        // The primary name should no longer be valid.
        assert_eq!(reverse_resolver.get_primary_name(&user), None);
    }

    #[test]
    fn test_set_new_primary_name_after_old_one_burned() {
        let (mut ctx, mut reverse_resolver) = setup();
        let (admin, user) = (ctx.admin, ctx.alice);

        // Register a name and set it as the primary name.
        ctx.with_name_registered(admin, user, TOKEN_TEST);
        ctx.set_caller(user);
        ctx.default_resolver
            .set_resolution(DOMAIN_TEST.to_string(), Some(user));
        reverse_resolver.set_primary_name(DOMAIN_TEST.to_string());
        assert_eq!(
            reverse_resolver.get_primary_name(&user),
            Some(DOMAIN_TEST.to_string())
        );

        // Burn the name backing the primary name. The primary name no longer
        // resolves, so it reads as None instead of reverting.
        ctx.admin_burn(vec![TOKEN_TEST]);
        assert_eq!(reverse_resolver.get_primary_name(&user), None);

        // Register a fresh name with a resolution and set it as the new primary
        // name. This must succeed even though the previous primary was burned.
        ctx.with_name_registered(admin, user, TOKEN_TEST2);
        ctx.set_caller(user);
        ctx.default_resolver
            .set_resolution(DOMAIN_TEST2.to_string(), Some(user));
        reverse_resolver.set_primary_name(DOMAIN_TEST2.to_string());
        assert_eq!(
            reverse_resolver.get_primary_name(&user),
            Some(DOMAIN_TEST2.to_string())
        );
    }

    #[test]
    fn test_set_primary_name_after_burn_and_remint() {
        let (mut ctx, mut reverse_resolver) = setup();
        let (admin, user) = (ctx.admin, ctx.alice);

        // Register a name, set its resolution and make it the primary name.
        ctx.with_name_registered(admin, user, TOKEN_TEST);
        ctx.set_caller(user);
        ctx.default_resolver
            .set_resolution(DOMAIN_TEST.to_string(), Some(user));
        reverse_resolver.set_primary_name(DOMAIN_TEST.to_string());
        assert_eq!(
            reverse_resolver.get_primary_name(&user),
            Some(DOMAIN_TEST.to_string())
        );

        // Burn the name. The primary name no longer resolves.
        ctx.admin_burn(vec![TOKEN_TEST]);
        assert_eq!(reverse_resolver.get_primary_name(&user), None);

        // Re-mint the same name. The fresh token has no resolution yet, so the
        // primary name still does not resolve.
        ctx.with_name_registered(admin, user, TOKEN_TEST);
        assert_eq!(reverse_resolver.get_primary_name(&user), None);

        // Set the resolution again and re-set the primary name. It succeeds.
        ctx.set_caller(user);
        ctx.default_resolver
            .set_resolution(DOMAIN_TEST.to_string(), Some(user));
        reverse_resolver.set_primary_name(DOMAIN_TEST.to_string());
        assert_eq!(
            reverse_resolver.get_primary_name(&user),
            Some(DOMAIN_TEST.to_string())
        );
    }

    fn setup() -> (TestContext, ReverseResolverHostRef) {
        let ctx = TestContext::install_and_setup();
        let reverse_resolver = ReverseResolver::deploy(
            &ctx.env,
            ReverseResolverInitArgs {
                name_token: ctx.token.address(),
            },
        );
        (ctx, reverse_resolver)
    }
}
