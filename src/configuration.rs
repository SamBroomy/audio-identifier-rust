use config::Config;
use reqwest::Url;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use serde_aux::field_attributes::deserialize_number_from_string;
use serde_with::{DurationMilliSeconds, serde_as};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions, PgSslMode},
};
use std::time::Duration;
use tokio::net::TcpListener;

use crate::domain::SubscriberEmail;

#[derive(Deserialize, Debug, Clone)]
pub struct Settings {
    #[serde(rename = "database")]
    pub database_cfg: DatabaseSettings,
    #[serde(rename = "application")]
    pub application_cfg: ApplicationSettings,
    #[serde(rename = "email_client")]
    pub email_client_cfg: EmailClientSettings,
}

impl Settings {
    pub fn new() -> Result<Self, config::ConfigError> {
        let base_path = std::env::current_dir().expect("Failed to determine the current directory");
        let configuration_directory = base_path.join("configuration");

        let environment: Environment = std::env::var("APP_ENVIRONMENT")
            .unwrap_or_else(|_| "local".into())
            .try_into()
            .expect("Failed to parse `APP_ENVIRONMENT`");

        let environment_filename = format!("{}.yml", environment.as_str());

        Config::builder()
            .add_source(config::File::from(configuration_directory.join("base.yml")))
            .add_source(
                config::Environment::with_prefix("APP")
                    .prefix_separator("_")
                    .separator("__"),
            )
            .add_source(config::File::from(
                configuration_directory.join(environment_filename),
            ))
            .build()?
            .try_deserialize()
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: SecretString,
    pub host: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub database_name: String,
    pub require_ssl: bool,
}

impl DatabaseSettings {
    pub fn connect_options(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            PgSslMode::Prefer
        };
        PgConnectOptions::new()
            .host(&self.host)
            .username(&self.username)
            .password(self.password.expose_secret())
            .port(self.port)
            .ssl_mode(ssl_mode)
            .database(&self.database_name)
    }

    pub fn connect_options_without_db(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            PgSslMode::Prefer
        };
        PgConnectOptions::new()
            .host(&self.host)
            .username(&self.username)
            .password(self.password.expose_secret())
            .port(self.port)
            .ssl_mode(ssl_mode)
    }

    pub fn get_pg_pool(&self) -> PgPool {
        PgPoolOptions::new()
            .acquire_timeout(std::time::Duration::from_secs(2))
            .connect_lazy_with(self.connect_options())
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct ApplicationSettings {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    #[serde(deserialize_with = "url_format::deserialize")]
    pub base_url: Url,
}

impl ApplicationSettings {
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
    pub async fn listener(&self) -> Result<TcpListener, std::io::Error> {
        TcpListener::bind(self.address()).await
    }
}
#[serde_as]
#[derive(Deserialize, Debug, Clone)]
pub struct EmailClientSettings {
    #[serde(deserialize_with = "url_format::deserialize")]
    pub base_url: Url,
    #[serde(deserialize_with = "subscriber_email_format::deserialize")]
    pub sender_email: SubscriberEmail,
    pub authorization_token: SecretString,
    #[serde_as(as = "DurationMilliSeconds<u64>")]
    pub timeout_ms: Duration,
}

pub enum Environment {
    Local,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "local" => Ok(Environment::Local),
            "production" => Ok(Environment::Production),
            other => Err(format!(
                "{} is not a supported environment. Use 'local' or 'production'.",
                other
            )),
        }
    }
}

// Add these modules for custom deserialization
mod url_format {
    use reqwest::Url;
    use serde::{Deserialize, Deserializer, de::Error};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Url, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Url::parse(&s).map_err(D::Error::custom)
    }
}

mod subscriber_email_format {
    use crate::domain::SubscriberEmail;
    use serde::{Deserialize, Deserializer, de::Error};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SubscriberEmail, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        SubscriberEmail::parse(s).map_err(D::Error::custom)
    }
}
