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
use crate::data_structures::{Payment, PaymentVoucher, RenewalPaymentVoucher};
use super::{registrar::RegistrarContractRef, utils::assert_voucher_not_expired};

#[odra::event]
pub struct PaymentFulfilled {
    payment_id: String,
    buyer: Address,
    amount: U512,
}

#[odra::module]
pub struct Controller {
    signer_public_key: Var<PublicKey>,
    registrar: External<RegistrarContractRef>,
    treasury: Var<Address>,
    access_control: SubModule<AccessControl>,
}

#[odra::module]
impl Controller {
    delegate! {
        to self.access_control {
            fn has_role(&self, role: &Role, address: &Address) -> bool;
            fn grant_role(&mut self, role: &Role, address: &Address);
            fn revoke_role(&mut self, role: &Role, address: &Address);
        }
    }

    pub fn init(&mut self, registrar: Address, signer: PublicKey, treasury: Address) {
        self.registrar.set(registrar);
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

    #[odra(payable)]
    pub fn buy(&mut self, voucher: PaymentVoucher, signature: Bytes) {
        assert_voucher_not_expired(&voucher, &self.env());
        self.assert_caller_is_buyer(&voucher);
        self.verify_signature(&voucher, &signature);
        self.collect_cspr_payment(&voucher);
        self.registrar.register(voucher.tokenization_vouchers);
    }

    #[odra(payable)]
    pub fn renew(&mut self, voucher: RenewalPaymentVoucher, signature: Bytes) {
        assert_voucher_not_expired(&voucher, &self.env());
        self.assert_caller_is_buyer(&voucher);
        self.verify_signature(&voucher, &signature);
        self.collect_cspr_payment(&voucher);
        self.registrar.renew(voucher.renewal_vouchers);
    }
}

impl Controller {
    fn _assert_caller_is_admin(&self) {
        self.access_control
            .check_role(&DEFAULT_ADMIN_ROLE, &self.env().caller());
    }

    fn assert_caller_is_buyer<P: Payment>(&self, voucher: &P) {
        if self.env().caller() != voucher.buyer() {
            self.revert(ControllerError::BuyerMustBeCaller);
        }
    }

    fn collect_cspr_payment<P: Payment>(&self, voucher: &P) {
        let fee_collector = self
            .treasury
            .get_or_revert_with(ControllerError::FeeCollectorNotSet);
        let price = voucher.price();
        self.env().transfer_tokens(&fee_collector, &price);
        self.env().emit_event(PaymentFulfilled {
            payment_id: voucher.payment_id(),
            buyer: voucher.buyer(),
            amount: price,
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
    InvalidSignature = 2001,
    FeeCollectorNotSet = 2002,
    RegistrarNotSet = 2003,
    BuyerMustBeCaller = 2004,
}

#[cfg(test)]
mod tests {
    use odra::{casper_types::U512, host::HostRef};

    use crate::{
        data_structures::{PaymentVoucher, TokenizationVoucher},
        test_context::TestContext,
    };

    #[test]
    fn test_controller() {
        let mut ctx = TestContext::install_and_setup();
        let (fee_collector, alice) = (ctx.treasury, ctx.alice);

        // Prepare a payment voucher.
        let expiration = ctx.expiration_time();
        let amount = U512::from(2000);
        let tokenization_voucher = TokenizationVoucher::new("label", expiration, alice, expiration);
        let voucher = PaymentVoucher::new(amount, "id_1", alice, vec![tokenization_voucher], expiration);
        let signature = ctx.sign(&voucher);

        // CSPR balances before the purchase.
        let fee_collector_balance = ctx.env.balance_of(&fee_collector);
        let alice_balance = ctx.env.balance_of(&alice);

        // But the voucher.
        ctx.env.set_caller(alice);
        ctx.controller
            .with_tokens(amount)
            .buy(voucher, signature.clone());

        // Token was minted.
        assert_eq!(ctx.token.balance_of(alice), 1);

        // CSPR balances after the purchase.
        assert_eq!(
            ctx.env.balance_of(&fee_collector),
            fee_collector_balance + amount
        );
        assert_eq!(ctx.env.balance_of(&alice), alice_balance - amount);
    }
}
