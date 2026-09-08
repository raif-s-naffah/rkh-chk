// SPDX-License-Identifier: GPL-3.0-or-later

use env_logger::WriteStyle::Never;
use jiff::Timestamp;
use log::{error, info};
use rkh_chk::{
    config::config,
    error::MyError,
    utils::{
        find_changed_files, find_dnf_history, find_offset, find_rpms, rkhunter_propupd_rpm,
        rkhunter_update, yes_no,
    },
};
use std::{env, io::Write, time::Instant};

/// IMPORTANT: MUST be run w/ `sudo`...
fn do_main() -> Result<(), MyError> {
    // 1. find offset of scan section to scrutinize...
    let offset = find_offset(env::args().collect())?;
    if offset == 0 {
        println!("→ Will scrutinize last scan section...");
    } else {
        println!("→ Will scrutinize scan section at offset -{}...", offset);
    }

    // 2. parse the target scan section and return all changed files within
    // that caused a warning about changed properties.
    let changed_files = find_changed_files(offset)?;
    info!(
        "Found {} file(s) with changed properties...",
        changed_files.len()
    );
    // if none were found we're done...
    if changed_files.is_empty() {
        return Ok(());
    }

    // 3. find RPMs providing those changed files.  remember that 1 RPM may be
    // providing more than one of those changed files...
    let outcome = find_rpms(changed_files)?;

    // 4. if at least one changed file was found to be unclaimed, invoke
    // `rkhunter --update --propupd` to ensure we're back in-sync w/ the master
    // database.  otherwise, invoke `rkhunter --propupd` for each changed RPM.
    if !outcome.unclaimed_files.is_empty() {
        info!(
            "Found unclaimed changed files: {:?}",
            outcome.unclaimed_files
        );
        if yes_no("Invoke rkhunter --update --propupd") {
            rkhunter_update()?;
            return Ok(());
        }
    }

    // 5. if we found no installed RPMs claiming _changed files_ or we did but
    // user declined calling `rkhunter --update --propupd`, we still have to deal
    // w/ installed RPMs that own _changed files_, if we found any.
    if !outcome.rpms.is_empty() {
        let rpms: Vec<String> = outcome.rpms.into_iter().collect();

        // try finding the last/latest DNF transaction that installed/updated
        // each RPM, inform the user + ask if they are ok w/ updating `rkhunter`
        // for each RPM.  we process all RPMs together to minimize the calls to
        // `dnf history info`.
        let dnf_history = find_dnf_history(offset, &rpms)?;
        // output details for the RPMs we'reinterested in only...
        dnf_history.print_details(&rpms);
        // ...then ask if they're ok calling rkhunter --propupd for that RPM
        for rpm in rpms {
            let prompt = format!("Invoke rkhunter update for package '{}'", rpm);
            if yes_no(&prompt) {
                rkhunter_propupd_rpm(&rpm)?;
            }
            println!()
        }
    }

    Ok(())
}

/// Setup logging from environment variables, including the `RUST_LOG` one
/// we included in our [MyConfig] singleton.
///
/// IMPORTANT - we also use a format for timestamps that shows (and uses) a
/// timezone which is the one detected as used by the system or the one
/// specified by the User in their `.env` configuration properties file.
fn setup_logging() -> Result<(), MyError> {
    let rust_log = &config().rust_log;
    let timezone = &config().timezone;
    let env = env_logger::Env::new().default_filter_or(rust_log);
    env_logger::builder()
        .parse_env(env)
        .format(move |buf, record| {
            let now = Timestamp::now();
            let zoned = now.in_tz(timezone).expect("Failed using configured TZ");
            let formatted = zoned.strftime("%Y-%m-%d %H:%M:%S %Z").to_string();
            writeln!(
                buf,
                "[{} {:>5}] {}",
                formatted,
                record.level(),
                record.args()
            )
        })
        .write_style(Never)
        .init();
    Ok(())
}

/// wrapper to measure how long this takes and tell us if anything unexpected
/// happened.
fn main() -> Result<(), MyError> {
    // print version string...
    println!("Version: {}", env!("CARGO_PKG_VERSION"));

    let now = Instant::now();

    // look for .env in same directory as this executable + load it...
    let mut env_path = env::current_exe().expect("Failed finding own path :(");
    let my_name = env_path
        .file_name()
        .expect("Failed finding own name :(")
        .to_str()
        .expect("Executable name has non UTF8 chars :(");
    let my_env = format!("{}.env", my_name);

    env_path.pop();
    env_path.push(my_env);
    println!("Will load .env from {:?}...\n", env_path);
    dotenvy::from_path_override(env_path)?;

    setup_logging()?;

    info!("Ready...");
    // the real McCoy...
    if let Err(x) = do_main() {
        error!("{}", x)
    }

    let elapsed = now.elapsed();
    println!("→ Done in {:.2?}", elapsed);
    Ok(())
}
