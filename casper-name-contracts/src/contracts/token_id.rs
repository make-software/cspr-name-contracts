use odra::{casper_types::U256, prelude::*};

pub trait ToTokenId {
    /// Converts the label to a token ID.
    fn token_id(&self, label: String) -> U256;
}

impl<T> ToTokenId for T
where
    T: Module,
{
    fn token_id(&self, label: String) -> U256 {
        let hash = self.env().hash(label);
        U256::from(hash)
    }
}
