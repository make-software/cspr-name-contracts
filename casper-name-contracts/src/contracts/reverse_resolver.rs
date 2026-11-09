use crate::contracts::{
    name_token::NameTokenContractRef, resolver::ResolverContractRef, token_id::ToTokenId, utils,
};
use odra::{prelude::*, ContractRef};
use odra_modules::dapp::DappContractBase;

#[odra::odra_error]
enum Error {
    InvalidTokenName = 1500,
    ResolutionForPrimaryNameNotFound = 1501,
    InvalidResolutionAddress = 1502,
    PrimaryNameNotSet = 1503,
    NotNameTokenOwner = 1504,
}

/// Reverse Resolver contract. It resolves primary names to addresses.
#[odra::module(events = [PrimaryNameChanged], errors = Error)]
pub struct ReverseResolver {
    primary_names: Mapping<Address, Option<String>>,
    name_token: External<NameTokenContractRef>,
    /// Membership in the CSPR.name dapp. The NameToken is the registry.
    dapp: SubModule<DappContractBase>,
}

#[odra::module]
impl ReverseResolver {
    delegate! {
        to self.dapp {
            /// Returns the dapp registry the contract belongs to.
            fn get_dapp_registry(&self) -> Address;
        }
    }

    pub fn init(&mut self, name_token: Address) {
        if !name_token.is_contract() {
            self.revert(Error::InvalidTokenName);
        }
        // The NameToken is also the dapp registry.
        self.name_token.set(name_token);
        self.dapp.set_dapp_registry(&name_token);
    }

    /// Runs once during an upgrade. Points the contract at its dapp registry,
    /// which is the NameToken contract.
    pub fn upgrade(&mut self) {
        let registry = *self.name_token.address();
        self.dapp.set_dapp_registry(&registry);
    }

    /// NameToken owner only. Changes the dapp registry the contract belongs to.
    pub fn set_dapp_registry(&mut self, registry: &Address) {
        if self.name_token.get_owner() != self.env().caller() {
            self.revert(Error::NotNameTokenOwner);
        }
        self.dapp.set_dapp_registry(registry);
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

    /// Clears the primary name of the caller.
    ///
    /// The forward resolution (name -> address) is left untouched, only the
    /// reverse record (address -> name) is removed. The stored primary name is
    /// cleared even if it no longer resolves to the caller (e.g. the name was
    /// burned), so a later re-mint of the same name does not revive it.
    ///
    /// Reverts with `PrimaryNameNotSet` if the caller has no primary
    /// name stored.
    pub fn unset_primary_name(&mut self) {
        let caller = self.env().caller();

        // Read the raw stored value, not the validated one, so a stale primary
        // name can be cleared too.
        let Some(old_primary_name) = self.primary_names.get(&caller).flatten() else {
            self.revert(Error::PrimaryNameNotSet);
        };

        self.primary_names.set(&caller, None);

        // Emit event.
        self.env().emit_event(PrimaryNameChanged {
            address: caller,
            old_primary_name: Some(old_primary_name),
            new_primary_name: None,
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
    use odra::host::{Deployer, HostRef};

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

        // `init` emits DappRegistryChanged, then one PrimaryNameChanged per call.
        assert_eq!(ctx.events_count(&reverse_resolver), 4);
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

    #[test]
    fn test_unset_primary_name() {
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

        // Unset the primary name.
        ctx.set_caller(user);
        reverse_resolver.unset_primary_name();

        // The primary name is gone.
        assert_eq!(reverse_resolver.get_primary_name(&user), None);

        // The forward resolution is still assigned to the user.
        assert_eq!(
            ctx.default_resolver.resolve(DOMAIN_TEST.to_string()),
            Some(user)
        );

        // DappRegistryChanged from `init`, then one event for set, one for unset.
        assert_eq!(ctx.events_count(&reverse_resolver), 3);
        assert_eq!(
            reverse_resolver.get_event::<PrimaryNameChanged>(-1),
            Ok(PrimaryNameChanged {
                address: user,
                old_primary_name: Some(DOMAIN_TEST.to_string()),
                new_primary_name: None,
            })
        );
    }

    #[test]
    fn test_unset_primary_name_when_not_set() {
        let (mut ctx, mut reverse_resolver) = setup();
        let user = ctx.alice;

        ctx.set_caller(user);
        let result = reverse_resolver.try_unset_primary_name();
        assert_eq!(result.unwrap_err(), Error::PrimaryNameNotSet.into());
        // Only the DappRegistryChanged event from `init`.
        assert_eq!(ctx.events_count(&reverse_resolver), 1);
    }

    #[test]
    fn test_unset_primary_name_twice() {
        let (mut ctx, mut reverse_resolver) = setup();
        let (admin, user) = (ctx.admin, ctx.alice);

        ctx.with_name_registered(admin, user, TOKEN_TEST);
        ctx.set_caller(user);
        ctx.default_resolver
            .set_resolution(DOMAIN_TEST.to_string(), Some(user));
        reverse_resolver.set_primary_name(DOMAIN_TEST.to_string());

        ctx.set_caller(user);
        reverse_resolver.unset_primary_name();

        // The second unset has nothing to clear.
        ctx.set_caller(user);
        let result = reverse_resolver.try_unset_primary_name();
        assert_eq!(result.unwrap_err(), Error::PrimaryNameNotSet.into());
    }

    #[test]
    fn test_unset_primary_name_only_affects_caller() {
        let (mut ctx, mut reverse_resolver) = setup();
        let (admin, user, other) = (ctx.admin, ctx.alice, ctx.bob);

        ctx.with_name_registered(admin, user, TOKEN_TEST);
        ctx.with_name_registered(admin, other, TOKEN_TEST2);

        ctx.set_caller(user);
        ctx.default_resolver
            .set_resolution(DOMAIN_TEST.to_string(), Some(user));
        reverse_resolver.set_primary_name(DOMAIN_TEST.to_string());

        ctx.set_caller(other);
        ctx.default_resolver
            .set_resolution(DOMAIN_TEST2.to_string(), Some(other));
        reverse_resolver.set_primary_name(DOMAIN_TEST2.to_string());

        // Unsetting as `other` leaves the user's primary name intact.
        ctx.set_caller(other);
        reverse_resolver.unset_primary_name();
        assert_eq!(reverse_resolver.get_primary_name(&other), None);
        assert_eq!(
            reverse_resolver.get_primary_name(&user),
            Some(DOMAIN_TEST.to_string())
        );
    }

    #[test]
    fn test_set_primary_name_again_after_unset() {
        let (mut ctx, mut reverse_resolver) = setup();
        let (admin, user) = (ctx.admin, ctx.alice);

        ctx.with_name_registered(admin, user, TOKEN_TEST);
        ctx.set_caller(user);
        ctx.default_resolver
            .set_resolution(DOMAIN_TEST.to_string(), Some(user));
        reverse_resolver.set_primary_name(DOMAIN_TEST.to_string());

        ctx.set_caller(user);
        reverse_resolver.unset_primary_name();
        assert_eq!(reverse_resolver.get_primary_name(&user), None);

        // The resolution is still there, so the name can be set as primary
        // again without touching the resolver.
        ctx.set_caller(user);
        reverse_resolver.set_primary_name(DOMAIN_TEST.to_string());
        assert_eq!(
            reverse_resolver.get_primary_name(&user),
            Some(DOMAIN_TEST.to_string())
        );
        assert_eq!(
            reverse_resolver.get_event::<PrimaryNameChanged>(-1),
            Ok(PrimaryNameChanged {
                address: user,
                old_primary_name: None,
                new_primary_name: Some(DOMAIN_TEST.to_string()),
            })
        );
    }

    #[test]
    fn test_unset_stale_primary_name_after_burn() {
        let (mut ctx, mut reverse_resolver) = setup();
        let (admin, user) = (ctx.admin, ctx.alice);

        ctx.with_name_registered(admin, user, TOKEN_TEST);
        ctx.set_caller(user);
        ctx.default_resolver
            .set_resolution(DOMAIN_TEST.to_string(), Some(user));
        reverse_resolver.set_primary_name(DOMAIN_TEST.to_string());

        // Burn the name. The primary name is stale but still stored.
        ctx.admin_burn(vec![TOKEN_TEST]);
        assert_eq!(reverse_resolver.get_primary_name(&user), None);

        // The stale primary name can still be cleared.
        ctx.set_caller(user);
        reverse_resolver.unset_primary_name();
        assert_eq!(
            reverse_resolver.get_event::<PrimaryNameChanged>(-1),
            Ok(PrimaryNameChanged {
                address: user,
                old_primary_name: Some(DOMAIN_TEST.to_string()),
                new_primary_name: None,
            })
        );

        // Re-mint the same name and restore its resolution. Since the primary
        // name was cleared, it does not come back on its own.
        ctx.with_name_registered(admin, user, TOKEN_TEST);
        ctx.set_caller(user);
        ctx.default_resolver
            .set_resolution(DOMAIN_TEST.to_string(), Some(user));
        assert_eq!(reverse_resolver.get_primary_name(&user), None);
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

#[cfg(test)]
mod dapp_tests {
    use super::*;
    use crate::test_context::TestContext;
    use odra::host::{Deployer, InstallConfig, NoArgs};

    #[test]
    fn test_dapp_registry_is_the_name_token() {
        let mut ctx = TestContext::install_and_setup();
        let token = ctx.token.address();
        let other = ctx.registrar.address();
        let mut reverse_resolver =
            ReverseResolver::deploy(&ctx.env, ReverseResolverInitArgs { name_token: token });
        assert_eq!(reverse_resolver.get_dapp_registry(), token);

        // Only the NameToken owner may re-point the contract.
        ctx.set_caller(ctx.alice);
        assert_eq!(
            reverse_resolver.try_set_dapp_registry(&other).unwrap_err(),
            Error::NotNameTokenOwner.into()
        );
        ctx.set_caller(ctx.admin);
        reverse_resolver.set_dapp_registry(&other);
        assert_eq!(reverse_resolver.get_dapp_registry(), other);
    }

    #[test]
    fn test_upgrade_points_at_the_name_token() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice) = (ctx.admin, ctx.alice);
        let token = ctx.token.address();
        let mut reverse_resolver = ReverseResolver::deploy_with_cfg(
            &ctx.env,
            ReverseResolverInitArgs { name_token: token },
            InstallConfig::upgradable::<ReverseResolver>(),
        );

        ctx.with_name_registered(admin, alice, "name");
        ctx.set_caller(alice);
        ctx.default_resolver
            .set_resolution("name.cspr".to_string(), Some(alice));
        reverse_resolver.set_primary_name("name.cspr".to_string());

        ctx.set_caller(admin);
        let upgraded =
            ReverseResolver::try_upgrade(&ctx.env, reverse_resolver.address(), NoArgs).unwrap();
        assert_eq!(upgraded.get_dapp_registry(), token);
        // Existing primary names survived.
        assert_eq!(
            upgraded.get_primary_name(&alice),
            Some("name.cspr".to_string())
        );
    }
}
