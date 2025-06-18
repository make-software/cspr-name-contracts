use super::registrar::RegistrarContractRef;
use crate::data_structures::{Payment, PaymentVoucher, RenewalPaymentVoucher};
use odra::{
    casper_types::{
        bytesrepr::{Bytes, ToBytes},
        PublicKey, U512,
    },
    prelude::*,
};
use odra_modules::{
    access::{AccessControl, Role, DEFAULT_ADMIN_ROLE},
    security::Pauseable,
};

/// Event with the payment information.
#[odra::event]
pub struct PaymentFulfilled {
    payment_id: String,
    buyer: Address,
    amount: U512,
}

/// Controller smart contract. It handles payments and talks to the [Registrar
/// Contract](super::registrar::Registrar).
#[odra::module]
pub struct Controller {
    controller: SubModule<BaseController>,
    registrar: External<RegistrarContractRef>,
}

#[odra::module]
impl Controller {
    delegate! {
        to self.controller {
            fn has_role(&self, role: &Role, address: &Address) -> bool;
            fn grant_role(&mut self, role: &Role, address: &Address);
            fn revoke_role(&mut self, role: &Role, address: &Address);
            fn set_signer_public_key(&mut self, signer: PublicKey);
            fn set_treasury(&mut self, treasury: Address);
            fn signer_public_key(&self) -> PublicKey;
            fn pause(&mut self);
            fn unpause(&mut self);
            fn is_paused(&self) -> bool;
        }
    }

    /// Initializes the controller with the registrar contract address, the
    /// signer public key and the treasury address.
    pub fn init(&mut self, registrar: Address, signer: PublicKey, treasury: Address) {
        self.registrar.set(registrar);
        self.controller.init(signer, treasury);
    }

    /// Payable. Buys new name tokens.
    #[odra(payable)]
    pub fn buy(&mut self, voucher: PaymentVoucher, signature: Bytes) {
        self.controller.require_not_paused();
        self.controller.process_payment_voucher(&voucher, signature);
        self.registrar.controller_register(voucher.into());
    }

    /// Payable. Renews name tokens.
    #[odra(payable)]
    pub fn renew(&mut self, voucher: RenewalPaymentVoucher, signature: Bytes) {
        self.controller.require_not_paused();
        self.controller.process_payment_voucher(&voucher, signature);
        self.registrar.controller_prolong(voucher.into());
    }

    /// Payable. Buys new name tokens and renews existing ones.
    #[odra(payable)]
    pub fn buy_and_renew(
        &mut self,
        payment_voucher: PaymentVoucher,
        payment_signature: Bytes,
        renewal_voucher: RenewalPaymentVoucher,
        renewal_signature: Bytes,
    ) {
        self.controller.require_not_paused();
        self.controller
            .process_payment_voucher(&payment_voucher, payment_signature);
        self.controller
            .process_payment_voucher(&renewal_voucher, renewal_signature);
        self.registrar
            .controller_prolong_and_register(renewal_voucher.into(), payment_voucher.into());
    }

    /// Try to resolve a full domain name to an address.
    pub fn resolve(&self, full_domain: String) -> Option<Address> {
        self.registrar.resolve(full_domain)
    }
}

/// Base for all controllers. It handles access controy, treasury and signer
/// public key.
#[odra::module(events = [PaymentFulfilled])]
pub struct BaseController {
    signer_public_key: Var<PublicKey>,
    treasury: Var<Address>,
    access_control: SubModule<AccessControl>,
    pausable: SubModule<Pauseable>,
}

#[odra::module]
impl BaseController {
    delegate! {
        to self.access_control {
            fn has_role(&self, role: &Role, address: &Address) -> bool;
            fn grant_role(&mut self, role: &Role, address: &Address);
            fn revoke_role(&mut self, role: &Role, address: &Address);
        }

        to self.pausable {
            fn is_paused(&self) -> bool;
            fn require_not_paused(&self);
        }
    }
}

impl BaseController {
    /// Initializes the controller.
    /// It assigns the deployer as the admin.
    pub fn init(&mut self, signer: PublicKey, treasury: Address) {
        self.signer_public_key.set(signer);
        self.treasury.set(treasury);

        // Setup roles.
        let admin = self.env().caller();
        self.access_control
            .unchecked_grant_role(&DEFAULT_ADMIN_ROLE, &admin);
    }

    /// Temporarily stops the contract.
    pub fn pause(&mut self) {
        self.assert_caller_is_admin();
        self.pausable.pause();
    }

    /// Returns to normal operation.
    pub fn unpause(&mut self) {
        self.assert_caller_is_admin();
        self.pausable.unpause();
    }

    /// Admin only. Sets the public key of the signer.
    pub fn set_signer_public_key(&mut self, signer: PublicKey) {
        self.assert_caller_is_admin();
        self.signer_public_key.set(signer);
    }

    /// Admin only. Sets the treasury address.
    pub fn set_treasury(&mut self, treasury: Address) {
        self.assert_caller_is_admin();
        self.treasury.set(treasury);
    }

    /// Returns the public key of the signer.
    pub fn signer_public_key(&self) -> PublicKey {
        self.signer_public_key.get().unwrap_or_revert(self)
    }

    #[inline]
    fn assert_caller_is_admin(&self) {
        self.access_control
            .check_role(&DEFAULT_ADMIN_ROLE, &self.env().caller());
    }

    fn assert_caller_is_buyer<P: Payment>(&self, voucher: &P) {
        if self.env().caller() != voucher.payment_info().buyer {
            self.revert(ControllerError::BuyerMustBeCaller);
        }
    }

    /// Validate the payment voucher and process the payment.
    pub fn process_payment_voucher<P: Payment + ToBytes>(&self, voucher: &P, signature: Bytes) {
        self.assert_caller_is_buyer(voucher);
        self.verify_signature(voucher, &signature);
        self.collect_cspr_payment(voucher);
    }

    fn collect_cspr_payment<P: Payment>(&self, voucher: &P) {
        let fee_collector = self
            .treasury
            .get_or_revert_with(ControllerError::FeeCollectorNotSet);
        let payment_info = voucher.payment_info();
        let attached_value = self.env().attached_value();
        if attached_value < payment_info.amount {
            self.revert(ControllerError::InsufficientPayment);
        }
        if attached_value > payment_info.amount {
            self.revert(ControllerError::PaymentTooLarge);
        }
        self.env()
            .transfer_tokens(&fee_collector, &payment_info.amount);
        self.env().emit_event(PaymentFulfilled {
            payment_id: payment_info.payment_id.clone(),
            buyer: payment_info.buyer,
            amount: payment_info.amount,
        });
    }

    fn verify_signature<T: ToBytes>(&self, data: &T, signature: &Bytes) {
        let public_key = self.signer_public_key();
        let bytes: Bytes = data.to_bytes().unwrap_or_revert(self).into();
        let verified = self.env().verify_signature(&bytes, signature, &public_key);
        if !verified {
            self.revert(ControllerError::InvalidSignature);
        }
    }
}

/// Controller errors.
#[odra::odra_error]
pub enum ControllerError {
    InvalidSignature = 1101,
    FeeCollectorNotSet = 1102,
    RegistrarNotSet = 1103,
    BuyerMustBeCaller = 1104,
    InsufficientPayment = 1105,
    PaymentTooLarge = 1106,
}

#[cfg(test)]
mod tests {
    use odra::{casper_types::U512, host::HostRef};

    use crate::{
        data_structures::{NameMintInfo, PaymentVoucher, RenewalPaymentVoucher, TokenRenewalInfo},
        test_context::{generate_token_id, TestContext, INIT_TIME, TOKEN_EXPIRATION, TOKEN_NAME},
    };

    #[test]
    fn test_controller() {
        let mut ctx = TestContext::install_and_setup();
        let (fee_collector, alice) = (ctx.treasury, ctx.alice);

        // Prepare a payment voucher.
        let token_expiration = ctx.token_expiration_time();
        let voucher_expiration = ctx.token_expiration_time();
        let amount = U512::from(2000);

        let names = vec![NameMintInfo::new(TOKEN_NAME, alice, token_expiration)];
        let voucher = PaymentVoucher::new(amount, "id_1", alice, names, voucher_expiration);
        let signature = ctx.sign(&voucher);

        // CSPR balances before the purchase.
        let fee_collector_balance = ctx.balance_of(&fee_collector);
        let alice_balance = ctx.balance_of(&alice);

        // But the voucher.
        ctx.set_caller(alice);
        ctx.controller
            .with_tokens(amount)
            .buy(voucher, signature.clone());

        // Token was minted.
        assert_eq!(ctx.token.balance_of(alice), 1.into());

        // CSPR balances after the purchase.
        assert_eq!(
            ctx.balance_of(&fee_collector),
            fee_collector_balance + amount
        );
        assert_eq!(ctx.balance_of(&alice), alice_balance - amount);
    }

    #[test]
    fn test_renew() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, fee_collector, alice) = (ctx.admin, ctx.treasury, ctx.alice);
        ctx.with_name_registered(admin, alice, TOKEN_NAME);

        // Prepare a payment voucher.
        let token_expiration = INIT_TIME + 2 * TOKEN_EXPIRATION;
        let voucher_expiration = INIT_TIME + TOKEN_EXPIRATION + 100;
        let amount = U512::from(2000);

        let names = vec![TokenRenewalInfo::new(
            generate_token_id(TOKEN_NAME),
            token_expiration,
        )];
        let voucher = RenewalPaymentVoucher::new(amount, "id_1", alice, names, voucher_expiration);
        let signature = ctx.sign(&voucher);

        // CSPR balances before the purchase.
        let fee_collector_balance = ctx.balance_of(&fee_collector);
        let alice_balance = ctx.balance_of(&alice);

        ctx.advance_block_time(TOKEN_EXPIRATION + 1);
        // But the voucher.
        ctx.set_caller(alice);
        ctx.controller
            .with_tokens(amount)
            .renew(voucher, signature.clone());

        // Token was minted.
        assert_eq!(ctx.token.balance_of(alice), 1.into());

        // CSPR balances after the purchase.
        assert_eq!(
            ctx.balance_of(&fee_collector),
            fee_collector_balance + amount
        );
        assert_eq!(ctx.balance_of(&alice), alice_balance - amount);
    }

    #[test]
    fn test_only_admin_can_pause() {
        let mut ctx = TestContext::install_and_setup();
        // Given a contract with an admin and a user.
        let (admin, alice) = (ctx.admin, ctx.alice);

        // When a non-admin tries to pause it.
        ctx.set_caller(alice);
        let result = ctx.controller.try_pause();
        // Then it should fail and the contract should not be paused.
        assert!(result.is_err());
        assert!(!ctx.controller.is_paused());

        // When the admin tries to pause it.
        ctx.set_caller(admin);
        ctx.controller.pause();

        // Then the contract should be paused.
        assert!(ctx.controller.is_paused());
    }

    #[test]
    fn test_only_admin_can_unpause() {
        let mut ctx = TestContext::install_and_setup();
        // Given a paused contract.
        let (admin, alice) = (ctx.admin, ctx.alice);
        ctx.controller.pause();
        assert!(ctx.controller.is_paused());

        // When a non-admin tries to unpause it.
        ctx.set_caller(alice);
        let result = ctx.controller.try_unpause();
        // Then it should fail.
        assert!(result.is_err());
        assert!(ctx.controller.is_paused());

        // When the admin tries to unpause it.
        ctx.set_caller(admin);
        ctx.controller.unpause();

        // Then the contract should be unpaused.
        assert!(!ctx.controller.is_paused());
    }

    #[test]
    fn test_buy_require_not_paused() {
        let mut ctx = TestContext::install_and_setup();
        // Given a paused contract.
        ctx.controller.pause();
        assert!(ctx.controller.is_paused());

        // When a user tries to buy a name.
        ctx.set_caller(ctx.alice);
        let voucher = PaymentVoucher::new(
            U512::from(2000),
            "id_1",
            ctx.alice,
            vec![NameMintInfo::new(
                TOKEN_NAME,
                ctx.alice,
                ctx.token_expiration_time(),
            )],
            ctx.token_expiration_time(),
        );
        let signature = ctx.sign(&voucher);
        let result = ctx.controller.try_buy(voucher, signature);

        // Then it should fail.
        assert_eq!(
            result,
            Err(odra_modules::security::errors::Error::UnpausedRequired.into())
        );
    }

    #[test]
    fn test_renew_require_not_paused() {
        let mut ctx = TestContext::install_and_setup();
        // Given a paused contract.
        ctx.controller.pause();
        assert!(ctx.controller.is_paused());

        // When a user tries to renew a name.
        ctx.set_caller(ctx.alice);
        let voucher = RenewalPaymentVoucher::new(
            U512::from(2000),
            "id_1",
            ctx.alice,
            vec![TokenRenewalInfo::new(
                generate_token_id(TOKEN_NAME),
                ctx.token_expiration_time(),
            )],
            ctx.token_expiration_time(),
        );
        let signature = ctx.sign(&voucher);
        let result = ctx.controller.try_renew(voucher, signature);

        // Then it should fail.
        assert_eq!(
            result,
            Err(odra_modules::security::errors::Error::UnpausedRequired.into())
        );
    }

    #[test]
    fn test_buy_and_renew_require_not_paused() {
        let mut ctx = TestContext::install_and_setup();
        // Given a paused contract.
        ctx.controller.pause();
        assert!(ctx.controller.is_paused());

        // When a user tries to buy and renew names.
        ctx.set_caller(ctx.alice);
        let payment_voucher = PaymentVoucher::new(
            U512::from(2000),
            "id_1",
            ctx.alice,
            vec![NameMintInfo::new(
                TOKEN_NAME,
                ctx.alice,
                ctx.token_expiration_time(),
            )],
            ctx.token_expiration_time(),
        );
        let renewal_voucher = RenewalPaymentVoucher::new(
            U512::from(2000),
            "id_2",
            ctx.alice,
            vec![TokenRenewalInfo::new(
                generate_token_id(TOKEN_NAME),
                ctx.token_expiration_time(),
            )],
            ctx.token_expiration_time(),
        );
        let payment_signature = ctx.sign(&payment_voucher);
        let renewal_signature = ctx.sign(&renewal_voucher);
        let result = ctx.controller.try_buy_and_renew(
            payment_voucher,
            payment_signature,
            renewal_voucher,
            renewal_signature,
        );

        // Then it should fail.
        assert_eq!(
            result,
            Err(odra_modules::security::errors::Error::UnpausedRequired.into())
        );
    }
}
