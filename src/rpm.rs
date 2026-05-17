// SPDX-License-Identifier: GPL-3.0-or-later

//! Artifacts and functions related to the [Red Hat Package Manager (RPM)][1]
//! used in this tool's context.
//! 
//! [1]: https://en.wikipedia.org/wiki/RPM_Package_Manager
//! 

use std::collections::HashSet;

use log::trace;

/// Variants representing the result of an `rpm -qf xxx` for a file
/// detected to have been changed by `rkhunter`...
pub enum RpmQueryOutcome {
    /// Variant representing base-name of an installed RPM that provides a
    /// designated changed file.
    OwnedBy(String),
    /// Variant representing the fact that a designated changed file was found
    /// not to have an installed RPM known to provide it, according to the
    /// `rkhunter` database.
    NotOwned,
}

/// When one or more _changed files_ were found in the designated _Scan Section_,
/// we try finding which RPM owns them.  this is the result of that investigation.
#[derive(Debug)]
pub struct FindRpmsOutcome {
    /// set of RPM full names claiming to own one or more _changed files_.
    pub rpms: HashSet<String>,
    /// number of times a _changed file_ was found NOT to be owned by any
    /// installed RPM.
    pub unclaimed_count: usize,
}

/// RPM package naming syntax: name-version-release.arch.  assuming 'version'
/// starts w/ a decimal digit, remove it + following parts returning `name`
/// only...
pub fn rpm_name(name: &str) -> String {
    trace!("rpm_name({})", name);
    // split on '-'
    let parts: Vec<&str> = name.split('-').collect();
    if parts.is_empty() {
        return name.to_owned();
    }

    let mut name_parts = Vec::new();
    for (ndx, part) in parts.iter().enumerate() {
        // if it starts w/ a decimal digit it's the `version` + everything
        // before is the 'name'..
        if part.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            name_parts = parts[..ndx].to_vec();
            break;
        }
    }

    // if no version found, use all parts.  should not happen with valid RPMs...
    if name_parts.is_empty() {
        name_parts = parts;
    }

    name_parts.join("-")
}
