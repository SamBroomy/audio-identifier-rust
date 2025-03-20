use core::fmt;

use validator::ValidateEmail;

#[derive(Debug, Clone)]
pub struct SubscriberEmail(String);

impl SubscriberEmail {
    pub fn parse(s: String) -> Result<Self, String> {
        if s.validate_email() {
            Ok(Self(s))
        } else {
            Err(format!("Failed to parse subscriber email: '{}'", s))
        }
    }
}

impl fmt::Display for SubscriberEmail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SubscriberEmail {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok};
    use fake::{Fake, faker::internet::en::SafeEmail};
    use proptest::prelude::*;
    use rand::{SeedableRng, rngs::StdRng};

    #[test]
    fn valid_emails_are_parsed_successfully() {
        let email = SafeEmail().fake();
        assert_ok!(SubscriberEmail::parse(email));
    }

    #[test]
    fn empty_string_is_rejected() {
        let email = "".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "ursula.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_domain_is_rejected() {
        let email = "ursula@".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    // Custom strategy for generating valid emails
    fn valid_email_strategy() -> impl Strategy<Value = String> {
        any::<u64>().prop_map(|seed| {
            let mut rng = StdRng::seed_from_u64(seed);
            SafeEmail().fake_with_rng(&mut rng)
        })
    }

    proptest! {
        #[test]
        fn prop_valid_emails_are_parsed_successfully(email in valid_email_strategy()) {
            prop_assert!(SubscriberEmail::parse(email).is_ok());
        }
    }
}
