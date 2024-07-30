use std::fmt::Debug;

use casper_name_contracts::data_structures::*;
use odra::{
    casper_types::{bytesrepr::ToBytes, U512},
    Address,
};

pub fn main() {
    print_struct(example_payment_info());
    print_struct(example_name_mint_info());
    print_struct(example_payment_voucher());
    print_struct(example_tokenization_voucher());
    print_struct(example_token_renewal_info());
    print_struct(example_renewal_payment_voucher());
    print_struct(example_renewal_voucher());
}

fn print_struct<T: ToBytes + Debug>(s: T) {
    println!("STRUCT:\n{:#?}\n", s);
    println!("HEX:\n{}\n\n", hex::encode(s.to_bytes().unwrap()));
}

fn example_payment_info() -> PaymentInfo {
    PaymentInfo {
        buyer: Address::new(
            "hash-15f4765f54755d38d0b191a01dbc08923f402e5da2251cb6a8a6bfec019ed05e",
        )
        .unwrap(),
        payment_id: String::from("test-payment-id"),
        amount: U512::from(100),
    }
}

fn example_name_mint_info() -> NameMintInfo {
    NameMintInfo {
        label: String::from("test-label"),
        owner: Address::new(
            "account-hash-38d0b191a01dbc0892338d0b191a01dbc08923f402e5da2251cb6a8a6bfec019",
        )
        .unwrap(),
        token_expiration: 123124,
    }
}

fn example_payment_voucher() -> PaymentVoucher {
    PaymentVoucher {
        payment: example_payment_info(),
        names: vec![example_name_mint_info(), example_name_mint_info()],
        voucher_expiration: 2435,
    }
}

fn example_tokenization_voucher() -> TokenizationVoucher {
    TokenizationVoucher {
        names: vec![example_name_mint_info(), example_name_mint_info()],
        voucher_expiration: 2435,
    }
}

fn example_token_renewal_info() -> TokenRenewalInfo {
    TokenRenewalInfo {
        token_id: String::from("test-token-id"),
        token_expiration: 999999,
    }
}

fn example_renewal_payment_voucher() -> RenewalPaymentVoucher {
    RenewalPaymentVoucher {
        payment: example_payment_info(),
        tokens: vec![example_token_renewal_info(), example_token_renewal_info()],
        voucher_expiration: 66666666,
    }
}

fn example_renewal_voucher() -> RenewalVoucher {
    RenewalVoucher {
        tokens: vec![example_token_renewal_info(), example_token_renewal_info()],
        voucher_expiration: 66666666,
    }
}
