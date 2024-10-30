use odra::{prelude::*, Address, Mapping};

/// Reverse Resolver contract. It resolves primary names to addresses.
#[odra::module(events = [PrimaryNameChanged])]
pub struct ReverseResolver {
    primary_names: Mapping<Address, String>,
}

#[odra::module]
impl ReverseResolver {
    /// Sets the primary preffered reverse resolution address for the caller.
    pub fn set_primary_name(&mut self, primary_name: String) {
        // Load currently set primary name.
        let caller = self.env().caller();
        let current_primary_name = self.get_primary_name(&caller);

        // Update primary name.
        self.primary_names.set(&caller, primary_name.clone());

        // Emit event.
        self.env().emit_event(PrimaryNameChanged {
            address: caller,
            old_primary_name: current_primary_name,
            new_primary_name: primary_name,
        });
    }

    /// Returns the primary name for the address.
    pub fn get_primary_name(&self, address: &Address) -> Option<String> {
        self.primary_names.get(address)
    }
}

/// Event emitted when the primary name of an address changes.
#[odra::event]
pub struct PrimaryNameChanged {
    pub address: Address,
    pub old_primary_name: Option<String>,
    pub new_primary_name: String,
}

#[cfg(test)]
mod tests {
    use odra::host::{Deployer, NoArgs};

    use super::*;

    #[test]
    fn test_set_primary_name() {
        let env = odra_test::env();
        let mut resolver = ReverseResolver::deploy(&env, NoArgs);

        let user = env.get_account(1);

        // It should have no primary name.
        assert_eq!(resolver.get_primary_name(&user), None);

        // Set primary name.
        env.set_caller(user);
        resolver.set_primary_name("test".to_string());

        // It should have the primary name.
        assert_eq!(resolver.get_primary_name(&user), Some("test".to_string()));

        // Set new primary name.
        env.set_caller(user);
        resolver.set_primary_name("test2".to_string());

        // It should have the new primary name.
        assert_eq!(resolver.get_primary_name(&user), Some("test2".to_string()));
    }
}
