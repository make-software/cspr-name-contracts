use super::registrar::RegistrarContractRef;
use crate::data_structures::{Payment, PaymentVoucher, RenewalPaymentVoucher};
use odra::{
    casper_types::{
        bytesrepr::{Bytes, ToBytes},
        PublicKey, U512,
    },
    module::{Module, Revertible},
    prelude::*,
    Address, External, SubModule, UnwrapOrRevert, Var,
};
use odra_modules::access::{AccessControl, Role, DEFAULT_ADMIN_ROLE};

#[odra::event]
pub struct PaymentFulfilled {
    payment_id: String,
    buyer: Address,
    amount: U512,
}

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
        }
    }

    pub fn init(&mut self, registrar: Address, signer: PublicKey, treasury: Address) {
        self.registrar.set(registrar);
        self.controller.init(signer, treasury);
    }

    pub fn set_signer_public_key(&mut self, signer: PublicKey) {
        self.controller.assert_caller_is_admin();
        self.controller.signer_public_key.set(signer);
    }

    pub fn set_treasury(&mut self, treasury: Address) {
        self.controller.assert_caller_is_admin();
        self.controller.treasury.set(treasury);
    }

    pub fn signer_public_key(&self) -> PublicKey {
        self.controller
            .signer_public_key
            .get()
            .unwrap_or_revert(self)
    }

    #[odra(payable)]
    pub fn buy(&mut self, voucher: PaymentVoucher, signature: Bytes) {
        self.controller.process_payment_voucher(&voucher, signature);
        self.registrar.register(voucher.into());
    }

    #[odra(payable)]
    pub fn renew(&mut self, voucher: RenewalPaymentVoucher, signature: Bytes) {
        self.controller.process_payment_voucher(&voucher, signature);
        self.registrar.prolong(voucher.into());
    }

    pub fn resolve(&self, full_domain: String) -> Option<Address> {
        self.registrar.resolve(full_domain)
    }
}

#[odra::module(events = [PaymentFulfilled])]
pub struct BaseController {
    signer_public_key: Var<PublicKey>,
    treasury: Var<Address>,
    access_control: SubModule<AccessControl>,
}

#[odra::module]
impl BaseController {
    delegate! {
        to self.access_control {
            fn has_role(&self, role: &Role, address: &Address) -> bool;
            fn grant_role(&mut self, role: &Role, address: &Address);
            fn revoke_role(&mut self, role: &Role, address: &Address);
        }
    }
}

impl BaseController {
    pub fn init(&mut self, signer: PublicKey, treasury: Address) {
        self.signer_public_key.set(signer);
        self.treasury.set(treasury);

        // Setup roles.
        let admin = self.env().caller();
        self.access_control
            .unchecked_grant_role(&DEFAULT_ADMIN_ROLE, &admin);
    }

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
        if self.env().attached_value() < payment_info.amount {
            self.revert(ControllerError::InsufficientPayment);
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

#[odra::odra_error]
pub enum ControllerError {
    InvalidSignature = 1101,
    FeeCollectorNotSet = 1102,
    RegistrarNotSet = 1103,
    BuyerMustBeCaller = 1104,
    InsufficientPayment = 1105,
}

#[cfg(test)]
mod tests {
    use odra::{casper_types::U512, host::HostRef};

    use crate::{
        data_structures::{NameMintInfo, PaymentVoucher, RenewalPaymentVoucher, TokenRenewalInfo},
        test_context::{TestContext, INIT_TIME, TOKEN_EXPIRATION, TOKEN_NAME},
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
        assert_eq!(ctx.token.balance_of(alice), 1);

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
            TOKEN_NAME.to_string(),
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
        assert_eq!(ctx.token.balance_of(alice), 1);

        // CSPR balances after the purchase.
        assert_eq!(
            ctx.balance_of(&fee_collector),
            fee_collector_balance + amount
        );
        assert_eq!(ctx.balance_of(&alice), alice_balance - amount);
    }
}
