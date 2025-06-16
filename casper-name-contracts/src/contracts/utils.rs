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

// It can be either:
// - value without .cspr like "odra"
// - value with .cspr like "odra.cspr"
// If this is contains subdomains, it returns false:
// - value without .cspr like "aaa.odra"
// - value with .cspr like "aaa.odra.cspr"
pub fn is_top_level_domain(full_domain: &str) -> bool {
    // Remove .cspr suffix if present
    let domain_without_suffix = if full_domain.ends_with(CSPR_DOMAIN) {
        full_domain.trim_end_matches(CSPR_DOMAIN)
    } else {
        full_domain
    };

    // Check if it's empty or contains dots (subdomains)
    !domain_without_suffix.is_empty() && !domain_without_suffix.contains('.')
}

#[cfg(test)]
mod t {
    use crate::contracts::utils::is_top_level_domain;

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

    #[test]
    fn test_is_top_level_domain() {
        // Correct values
        let correct = [
            "odra",
            "odra.cspr",
            "cspr",
            "cspr.cspr",
        ];
        for domain in correct {
            assert!(is_top_level_domain(domain), "Expected '{}' to be a top-level domain", domain);
        }
        // Incorrect values
        let incorrect = [
            "aaa.odra",
            "aaa.odra.cspr",
            "ss.aaa.odra.cspr",
            "ss.aaa.odra",
            "ss.aaa.odra.csp",
            ".cspr"
        ];
        for domain in incorrect {
            assert!(!is_top_level_domain(domain), "Expected '{}' to not be a top-level domain", domain);
        }
    }
}
