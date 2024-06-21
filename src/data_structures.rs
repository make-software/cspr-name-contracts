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
    pub label: String,
    pub expiration: u64,
}

impl NameTokenMetadata {
    pub fn new(label: &str, expiration: u64) -> Self {
        Self {
            label: String::from(label),
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
        Self::new(&voucher.label, voucher.expiration)
    }
}

#[odra::odra_type]
pub struct TokenizationVoucher {
    pub label: String,
    pub expiration: u64,
    pub buyer: Address,
}

impl TokenizationVoucher {
    pub fn new(label: &str, expiration: u64, buyer: Address) -> Self {
        Self {
            label: String::from(label),
            expiration,
            buyer,
        }
    }
}

#[odra::odra_type]
pub struct PaymentVoucher {
    pub tokenization_voucher: TokenizationVoucher,
    pub price: U512,
}

impl PaymentVoucher {
    pub fn new(label: &str, expiration: u64, buyer: Address, price: U512) -> Self {
        Self {
            tokenization_voucher: TokenizationVoucher::new(label, expiration, buyer),
            price,
        }
    }
}

pub struct RenewalVoucher {
    pub token_hash: String,
    pub expiration: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_serialization() {
        let expected = r#"{
            "label": "test-label",
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
