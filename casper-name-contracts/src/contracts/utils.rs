use odra::prelude::*;

const CSPR_DOMAIN: &str = ".cspr";

// pub fn to_utf8_string(hash: &[u8]) -> Option<String> {
//     let mut result = [0u8; 64];
//     odra::utils::hex_to_slice(hash, &mut result);
//     String::from_utf8(result.to_vec()).ok()
// }

pub fn extract_token_name(full_domain: &str) -> Option<String> {
    if full_domain.ends_with(CSPR_DOMAIN) {
        let token_name = full_domain.trim_end_matches(CSPR_DOMAIN);
        token_name.split('.').last().map(|s| s.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod t {
    #[test]
    fn test_extract_token_name() {
        let full_domain = "odra.cspr";
        let token_name = super::extract_token_name(full_domain).unwrap();
        assert_eq!(token_name, "odra");

        let full_domain = "aaa.odra.cspr";
        let token_name = super::extract_token_name(full_domain).unwrap();
        assert_eq!(token_name, "odra");

        let full_domain = "ss.aaa.odra.cspr";
        let token_name = super::extract_token_name(full_domain).unwrap();
        assert_eq!(token_name, "odra");

        let full_domain = "aaa.odra.csp";
        let token_name = super::extract_token_name(full_domain);
        assert_eq!(token_name, None);
    }
}
