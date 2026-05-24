// SPDX-License-Identifier: GPL-3.0-or-later

//! Artifacts and functions related to configuring the tool.

use crate::error::MyError;
use jiff::tz::TimeZone;
use std::sync::OnceLock;

const DEFAULT_MAX_OFFSET: usize = 10;
const DEFAULT_MAX_DNF_DEJA_VU: usize = 2;
const DEFAULT_RUST_LOG: &str = "info";
const UTC_TZ_NAME: &str = "utc";

static CONFIG: OnceLock<MyConfig> = OnceLock::new();
/// Configuration Singleton.
pub fn config() -> &'static MyConfig {
    CONFIG.get_or_init(|| MyConfig::new().expect("Failed loading configuration"))
}

/// Macro to deal w/ configuration paramters of type `usize`.
macro_rules! load_uint_config_param {
    ($param_name:expr, $default_val:expr) => {{
        match dotenvy::var($param_name) {
            Ok(x) => x.parse().expect(&format!("Failed parsing {}", $param_name)),
            Err(x) => {
                ::log::warn!(
                    "{} is not set or is invalid ({}). Will use {} instead",
                    $param_name,
                    x,
                    $default_val
                );
                $default_val
            }
        }
    }};
}

/// Structure providing configuration parameters loaded at startup from
/// environment variables.
#[derive(Debug)]
pub struct MyConfig {
    /// `MAX_OFFSET` - Unsigned decimal integer, less or equal to 10 indicating
    /// the maximum allowed value for OFFSET. Default is 10.
    pub max_offset: usize,

    /// `MAX_DNF_DEJA_VU` - Unsigned decimal integer, less or equal to 3
    /// indicating the maximum number of DNF History Info frames we'll fetch
    /// earlier than the user-provided OFFSET index. Default is 2.
    pub max_dnf_deja_vu: usize,

    /// `DEFAULT_SYSTEM_TZ` - DNF History frames date-times are expressed in
    /// UTC timezone. The tool tries to display them in the system's one. If
    /// however it fails to discover that name, it will use this default. If
    /// it is missing or misconfigured, date-times will be kept in UTC.
    pub timezone: String,

    /// `RUST_LOG` - Log verbosity.  Valid values are: "OFF", "ERROR", "WARN",
    /// "INFO", "DEBUG", "TRACE". Defaults to "info".
    pub rust_log: String,
}

impl MyConfig {
    // emit messages to stderr since logging at this stage is not yet set...
    fn new() -> Result<Self, MyError> {
        let max_offset = load_uint_config_param!("MAX_OFFSET", DEFAULT_MAX_OFFSET);
        let max_dnf_deja_vu = load_uint_config_param!("MAX_DNF_DEJA_VU", DEFAULT_MAX_DNF_DEJA_VU);

        // try finding out the system's timezone.  if successful use it.  if not
        // look for DEFAULT_SYSTEM_TZ environment variable provided and use that
        // if set.  if that too fails, fall back on UTC.
        let system_tz = TimeZone::system();
        let system_tz_name = system_tz.iana_name();
        let (check_user_override, timezone) = match system_tz_name {
            Some(x) => {
                // ensure discovered name yields a valid TimeZone instance...
                let _ = TimeZone::get(x)?;
                (false, x)
            }
            None => (true, ""),
        };
        let timezone = if check_user_override {
            match dotenvy::var("DEFAULT_SYSTEM_TZ") {
                Ok(x) => {
                    // same precaution.  ensure it yields a valid TimeZone instance...
                    match TimeZone::get(&x) {
                        Ok(_) => x,
                        Err(y) => {
                            eprintln!("✘ Failed parsing DEFAULT_SYSTEM_TZ ({}): {}. Use UTC", x, y);
                            UTC_TZ_NAME.to_owned()
                        }
                    }
                }
                Err(x) => {
                    eprintln!("✘ Failed loading DEFAULT_SYSTEM_TZ. Use UTC: {}", x);
                    UTC_TZ_NAME.to_owned()
                }
            }
        } else {
            timezone.to_owned()
        };

        let rust_log = match dotenvy::var("RUST_LOG") {
            Ok(x) => x,
            Err(x) => {
                eprintln!("✘ Failed loading RUST_LOG. Use default: {}", x);
                DEFAULT_RUST_LOG.to_owned()
            }
        };

        Ok(Self {
            max_offset,
            max_dnf_deja_vu,
            timezone,
            rust_log,
        })
    }
}
