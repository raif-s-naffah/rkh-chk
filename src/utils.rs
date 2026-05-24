// SPDX-License-Identifier: GPL-3.0-or-later

//! Collection of utility functions used throughout this tool.
//!

use crate::{
    config::config,
    dnf::DnfHistoryFrames,
    error::MyError,
    rpm::{FindRpmsOutcome, RpmQueryOutcome, rpm_name},
};
use log::{debug, error, info, trace, warn};
use std::{
    collections::HashSet,
    fs::File,
    io::{BufRead, BufReader, Write, stdin, stdout},
    process::Command,
};

/// possible error messages...
const INVALID_OFFSET: &str = "Invalid offset. Use 'last', 'last-1', 'last-2', and so on...";
const OFFSET_OOB: &str = "Out of bounds offset";
const SCAN_SECTION_NOT_FOUND: &str = "No Start Scan marker found at designated offset. Break...";

/// where rkhunter log file lives...
const RKHUNTER_LOG: &str = "/var/log/rkhunter/rkhunter.log";

/// Message fragment in `rpm -qf` output when file is not owned by any RPM.
/// this is the case for some symlinks in `/usr/sbin`...
const UNCLAIMED_MARKER: &str = "is not owned by any package";

/// markers that delimit RKH scan sections...
const START_SCAN_MARKER: &str = "Start Rootkit Hunter Scan";
const END_SCAN_MARKER: &str = "End Rootkit Hunter Scan";

/// warning marker for unexpected file properties change...
const WARN_FILE_CHANGED_MARKER: &str = "Warning: The file properties have changed:";

/// check `args`.  only 1 is expected to let us know what is the target OFFSET
/// of the invocation of the tool.
///
/// If it's missing or, ignoring case, is equal to 'last' then `0` is returned.
/// Otherwise, it's assumed to be of the form 'last-N' where `N` is a decimal
/// number between 1 and MAX_OFFSET.
///
/// Retrn error if parsing the argument fails, the 1st argument was present
/// but was invalid (not of the expected form), or is greater than the maximum
/// allowed value.
pub fn find_offset(args: Vec<String>) -> Result<usize, MyError> {
    trace!("find_offset({:?})", args);
    let offset = if args.len() > 1 {
        let it = args[1].to_lowercase();
        if it == "last" {
            0
        } else if it.starts_with("last-") {
            it.strip_prefix("last-").unwrap().parse::<usize>()?
        } else {
            return Err(MyError::Runtime(INVALID_OFFSET.to_owned()));
        }
    } else {
        info!("Missing OFFSET. Assume 0 + continue...");
        0
    };
    if offset > config().max_offset {
        Err(MyError::Runtime(OFFSET_OOB.to_owned()))
    } else {
        Ok(offset)
    }
}

/// parse the Scan Section at `offset` and return a collection of file paths
/// that caused a warning in the log.
pub fn find_changed_files(offset: usize) -> Result<Vec<String>, MyError> {
    trace!("find_changed_files({})", offset);
    // open + read the log file into memory...
    let file = File::open(RKHUNTER_LOG)?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().collect::<Result<_, _>>()?;

    // find target scan section, iterating in reverse order.  effectively find
    // line index of the OFFSET-th START marker...
    let mut count = 0;
    let mut start_ndx = 0;
    let mut found = false;
    for (ndx, line) in lines.iter().enumerate().rev() {
        if line.contains(START_SCAN_MARKER) {
            if count == offset {
                start_ndx = ndx;
                found = true;
                break;
            }
            count += 1;
        }
    }

    if !found {
        return Err(MyError::Runtime(SCAN_SECTION_NOT_FOUND.to_owned()));
    }

    // skip all lines before found desired start scan section index looking for
    // warnings about changed file properties w/in the section...
    let mut changed_files = Vec::new();
    let mut found_warning = false;
    for line in lines.iter().skip(start_ndx) {
        if line.contains(END_SCAN_MARKER) {
            break;
        }

        if line.contains(WARN_FILE_CHANGED_MARKER) {
            found_warning = true;
            continue;
        }

        // once we find a warning, the following line should contain the path...
        if found_warning {
            let trimmed = line.trim_start();
            if trimmed.starts_with("File: ") {
                let file_path = trimmed.strip_prefix("File: ").unwrap().trim();
                changed_files.push(file_path.to_string());
            }
            found_warning = false;
        }
    }

    Ok(changed_files)
}

/// given a collection of _changed _files_ call `rpm -qf` on each of them and
/// return the result.
pub fn find_rpms(files: Vec<String>) -> Result<FindRpmsOutcome, MyError> {
    trace!("find_rpms({:?})", files);
    let mut rpms = HashSet::new();
    let mut unclaimed_files = HashSet::new();
    for file in files {
        match rpm_query(&file) {
            Ok(RpmQueryOutcome::OwnedBy(x)) => rpms.insert(x),
            Ok(RpmQueryOutcome::NotOwned(x)) => unclaimed_files.insert(x),
            Err(x) => return Err(x),
        };
    }
    Ok(FindRpmsOutcome {
        rpms,
        unclaimed_files,
    })
}

/// issue an `rpm -qf` to find which installed RPM provides the given file.
/// there must be one and only one such RPM!  note also that some _changed
/// files_ may not be provided by any RPM, e.g. some symlinks under
/// `/usr/sbin`.  for example...
/// ```bash
///   $ rpm -qf /usr/sbin/grpck
///   file /usr/sbin/grpck is not owned by any package
/// ```
/// those files are properly handled by issuing an `rkhunter --propupd` which
/// btw. should also take care of all changed files that are OWNED by an RPM.
///
/// return an error if the command fails, or `file` was found to be provided by
/// more than one installed RPM.
///
/// IMPORTANT - when an installed RPM package is found to own the designated
/// file, we store its _base name_ stripping release, version, arch, etc...
/// parts from the full name.
pub fn rpm_query(file: &str) -> Result<RpmQueryOutcome, MyError> {
    trace!("rpm_query({})", file);
    match run_cmd_lenient("rpm", &["-qf", file]) {
        Ok(x) => {
            // ensure we found 1 provider RPM...
            if x.len() != 1 {
                let msg = format!(
                    "Wasn't expecting more than 1 RPM providing `{}`; found {}. Break..",
                    file,
                    x.len()
                );
                error!("{}", msg);
                Err(MyError::Runtime(msg))
            } else {
                // got 1-line output.  it can be either the providing full RPM
                // or a message that the file is NOT owned by any RPM...
                let line = &x[0];
                if line.ends_with(UNCLAIMED_MARKER) {
                    Ok(RpmQueryOutcome::NotOwned(file.to_owned()))
                } else {
                    Ok(RpmQueryOutcome::OwnedBy(rpm_name(line)))
                }
            }
        }
        Err(x) => {
            warn!("Failed rpm -qf {}: {}", file, x);
            Err(x)
        }
    }
}

/// invoke a given `cmd`, optionally w/ argument(s).  if it succeeds, collect
/// `stdout` output and return it as a `Vec<String>`.  if it fails, emit an
/// error w/ what was sent to `stderr`.
///
/// IMPORTANT (rsn) 20260506 - sometimes an `rpm -qf xxx` command returns a FALSE
/// status --implying a failure-- but effectively the file argument was not found
/// to be owned by any installed RPM.  in other words, the output of `stdout` IS
/// the right/expected answer.  for this reason we use a `strict` boolean argument
/// to let us know how to behave in those cases --read the source Luke...
pub fn run_cmd_strict(cmd: &str, args: &[&str]) -> Result<Vec<String>, MyError> {
    run_cmd(cmd, args, true)
}
pub fn run_cmd_lenient(cmd: &str, args: &[&str]) -> Result<Vec<String>, MyError> {
    run_cmd(cmd, args, false)
}
fn run_cmd(cmd: &str, args: &[&str], strict: bool) -> Result<Vec<String>, MyError> {
    debug!("run_cmd({}, {:?}, strict? {})", cmd, args, strict);
    let output = Command::new(cmd).args(args).output()?;
    if !output.status.success() && strict {
        let err = String::from_utf8_lossy(&output.stderr);
        error!("Failed command ('{}'): {:?}", cmd, err);
        return Err(MyError::Command((String::from(cmd), String::from(err))));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<String> = stdout
        .lines()
        .map(|x| {
            trace!("<<< {}", x);
            String::from(x)
        })
        .collect();
    Ok(lines)
}

/// invoke `rkhunter --update --propupd` ensuring that local DB is in sync w/
/// rkhunter's master...
/// return error if invoking the command failed.
pub fn rkhunter_update() -> Result<Vec<String>, MyError> {
    // if yes_no("Invoke rkhunter --update --propupd") {
    info!("About to update rkhunter. This may take a while...");
    run_cmd("rkhunter", &["--update", "--propupd"], false)
}

/// prompt user and wait for a yes/no answer w/ 'No' being the default.
pub fn yes_no(prompt: &str) -> bool {
    loop {
        print!("> {}. Is this ok [y/N]? ", prompt);
        stdout().flush().unwrap();

        let mut input = String::new();
        stdin().read_line(&mut input).unwrap();

        match input.trim().to_lowercase().as_str() {
            "y" | "yes" => return true,
            "n" | "no" | "" => return false,
            _ => println!("> A 'y|Y[es]' or 'n|N[o]' answer is expected. Try again..."),
        }
    }
}

/// invoke `rkhunter --propupd xxx` targeting a specific RPM, given its base
/// name excl. release, version, architecture, etc...
pub fn rkhunter_propupd_rpm(rpm: &str) -> Result<(), MyError> {
    info!(
        "About to run 'rkhunter --propupd {}'. Shouldn't take long...",
        rpm
    );
    match run_cmd("rkhunter", &["--propupd", rpm], false) {
        Ok(_) => Ok(()),
        Err(x) => Err(x),
    }
}

/// Return a collection of [DnfHistoryFrames] (which contains a collection of
/// [DnfHistory][1] frames whose [DnfEvent][2] contain a mention
/// of any of the designated RPMs.
///
/// [1]: crate::dnf::DnfHistory
/// [2]: crate::dnf::DnfEvent
pub fn find_dnf_history(offset: usize, rpms: &Vec<String>) -> Result<DnfHistoryFrames, MyError> {
    trace!("find_dnf_history({}, {:?})", offset, rpms);
    // where we store the outcome of calling `dnf history info xxx` so we only
    // do it once for all RPMs we're investigating.
    //
    // it's a Vec that holds maximum allowed instances of DNF History Info records
    // we need to check to find latest transaction pertaining to our installed RPMs
    // owning _changed files_.  the first will be the result of `dnf history info
    // last-offset`, followed by the the one from `dnf history info last-offset-1`
    // etc...  in other words the element at index N shall correspond to the DNF
    // History Info for `last-offset-N`.
    let mut dnf_hi_cache = DnfHistoryFrames::default();

    // need to build a collection of dnf history info summaries starting from
    // `last-offset` and potentially going further back
    for rpm in rpms {
        let mut cache_ndx = 0;
        loop {
            if cache_ndx >= dnf_hi_cache.len() {
                dnf_hi_cache = dnf_hi_cache.grow(offset)?;
            }

            if dnf_hi_cache.contains_rpm(rpm, cache_ndx) {
                break;
            }

            cache_ndx += 1;
            if cache_ndx > config().max_dnf_deja_vu {
                warn!(
                    "MAX_DNF_DEJA_VU reached w/o finding '{}' in past transactions",
                    rpm
                );
                break;
            }
        }
    }

    Ok(dnf_hi_cache)
}
