use odra::{casper_types::U512, prelude::*, Address};
use serde::{Deserialize, Serialize};

#[odra::odra_error]
#[derive(Debug)]
pub enum NameTokenError {
    EmptyTLD = 1001,
    TLDNotSupported = 1002,
    PastExpirationDate = 1003,
    EmptyLabel = 1004,
    SLDDoesNotExist = 1005,
    SerializationError = 1006,
    DeserializationError = 1007,
}

#[odra::odra_type]
#[derive(Serialize, Deserialize)]
pub struct NameTokenMetadata {
    pub token_hash: String,
    pub expiration: u64,
}

impl NameTokenMetadata {
    pub fn new(token_hash: &str, expiration: u64) -> Self {
        Self {
            token_hash: String::from(token_hash),
            expiration,
        }
    }

    pub fn to_json(&self) -> Result<String, NameTokenError> {
        serde_json_wasm::to_string(self).map_err(|_| NameTokenError::SerializationError)
    }

    pub fn from_json(json: &str) -> Result<Self, NameTokenError> {
        serde_json_wasm::from_str(json).map_err(|_| NameTokenError::DeserializationError)
    }
}

impl From<&TokenizationVoucher> for NameTokenMetadata {
    fn from(voucher: &TokenizationVoucher) -> Self {
        Self::new(&voucher.token_hash, voucher.token_expiration)
    }
}

#[odra::odra_type]
pub struct TokenizationVoucher {
    pub token_hash: String,
    pub owner: Address,
    pub token_expiration: u64,
    pub voucher_expiration: u64,
}

impl TokenizationVoucher {
    pub fn new(
        token_hash: &str,
        owner: Address,
        token_expiration: u64,
        voucher_expiration: u64,
    ) -> Self {
        Self {
            token_hash: String::from(token_hash),
            owner,
            token_expiration,
            voucher_expiration,
        }
    }
}

#[odra::odra_type]
pub struct PaymentVoucher {
    pub tokenization_vouchers: Vec<TokenizationVoucher>,
    pub price: U512,
    pub payment_id: String,
    pub buyer: Address,
    pub voucher_expiration: u64, // TODO: support this, test it.
}

impl PaymentVoucher {
    pub fn new(
        price: U512,
        payment_id: &str,
        buyer: Address,
        vouchers: Vec<TokenizationVoucher>,
        voucher_expiration: u64,
    ) -> Self {
        Self {
            tokenization_vouchers: vouchers,
            price,
            payment_id: String::from(payment_id),
            buyer,
            voucher_expiration,
        }
    }
}

#[odra::odra_type]
pub struct RenewalPaymentVoucher {
    pub renewal_vouchers: Vec<RenewalVoucher>,
    pub price: U512,
    pub payment_id: String,
    pub buyer: Address,
    pub voucher_expiration: u64,
}

#[odra::odra_type]
pub struct RenewalVoucher {
    pub token_hash: String,
    pub token_expiration: u64,
    pub voucher_expiration: u64,
}

pub trait Payment {
    fn price(&self) -> U512;
    fn payment_id(&self) -> String;
    fn buyer(&self) -> Address;
}

impl Payment for PaymentVoucher {
    fn price(&self) -> U512 {
        self.price
    }

    fn payment_id(&self) -> String {
        self.payment_id.clone()
    }

    fn buyer(&self) -> Address {
        self.buyer
    }
}

impl Payment for RenewalPaymentVoucher {
    fn price(&self) -> U512 {
        self.price
    }

    fn payment_id(&self) -> String {
        self.payment_id.clone()
    }

    fn buyer(&self) -> Address {
        self.buyer
    }
}

pub trait ExpirableVoucher {
    fn expiration_time(&self) -> u64;
}

impl ExpirableVoucher for RenewalPaymentVoucher {
    fn expiration_time(&self) -> u64 {
        self.voucher_expiration
    }
}

impl ExpirableVoucher for PaymentVoucher {
    fn expiration_time(&self) -> u64 {
        self.voucher_expiration
    }
}

impl ExpirableVoucher for TokenizationVoucher {
    fn expiration_time(&self) -> u64 {
        self.voucher_expiration
    }
}

impl ExpirableVoucher for RenewalVoucher {
    fn expiration_time(&self) -> u64 {
        self.voucher_expiration
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_serialization() {
        let expected = r#"{
            "token_hash": "test-label",
            "expiration": 86400
        }"#
        .replace(" ", "")
        .replace("\n", "");

        // Test metadata to_json.
        let metadata = NameTokenMetadata::new("test-label", 86400);
        assert_eq!(expected, metadata.to_json().unwrap());

        // Test metadata from_json.
        let deserialized = NameTokenMetadata::from_json(&expected).unwrap();
        assert_eq!(metadata, deserialized);
    }
}
