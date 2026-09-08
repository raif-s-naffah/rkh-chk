// SPDX-License-Identifier: GPL-3.0-or-later

#![warn(missing_docs)]

//!
//! # How does it work?
//! 1. The tool looks for one command line argument to compute an `OFFSET` parameter.  When that
//!    argument is missing, `OFFSET` is set to `0`.  When present, it's expected to be of the form
//!    `last[-N]` (in the style of the DNF History Info command argument), with `N` being a
//!    decimal number between `1` and `MAX_OFFSET` (defaults to `10`).
//!
//! 2. It then scans the latest `rkhunter` log to find the _Scan Section_ at 0-based `OFFSET` index,
//!    starting from the end.  In other words, `OFFSET` 0 is the _last_ section in the log file.
//!    Here's an example of such a section:
//!    ```text
//!     ---------------------- Start Rootkit Hunter Scan ----------------------
//!     Warning: The file properties have changed:
//!              File: /usr/bin/less
//!              Current inode: 5249925    Stored inode: 5245064
//!
//!     ----------------------- End Rootkit Hunter Scan -----------------------
//!     ```
//!
//! 3. If the desired section is found, it extracts the collection of _changed files_ mentioned w/in
//!    the section. Continuing w/ the example above, one _changed file_ is mentioned: `/usr/bin/less`.
//!
//! 4. It then queries the RPM database to find which package _"owns"_ that file. It does that by
//!    invoking `rpm -qf ` using the changed file path as argument.  If an installed RPM _owns_ the
//!    file, the result will be something like this:
//!     ```text
//!     $ rpm -qf /usr/bin/less↵
//!     less-692-6.fc44.x86_64
//!     ```
//!    On the other hand, if that _changed file_ is found NOT to be owned by any installed RPM, the 
//!    result of the query will be something like this...
//!     ```text
//!     $ rpm -qf /usr/sbin/grpck↵
//!     file /usr/sbin/grpck is not owned by any package
//!     ```
//!    Such files are called _unclaimed_ &mdash;they're usually symlinks to same named files in `/usr/bin`.
//!    Those files are not tracked by `rkhunter`.
//!
//! 5. At this point, the tool will have gathered the set of _changed RPMs_; i.e. installed RPMs that
//!    own the _changed files_, as well as the _unclaimed_ ones.  Since "fixing" _unclaimed files_
//!    can only be done by updating the entire `rkhunter` database, if the _count of unclaimed files_
//!    is different than zero, the tool will offer the User a yes/no prompt to do that.  If the User
//!    answers in the affirmative, the tool will invoke `rkhunter --update --propupd` and exits when
//!    done.
//!
//! 6. If the _count of unclaimed files_ is zero, or was not but User declined to update `rkhunter`,
//!    the tool will proceed to process the _changed RPMs_.
//!
//! 7. Processing _changed RPMs_ starts by collecting, at most `MAX_DNF_DEJA_VU`, _DNF History Frames_
//!    that  mention any _changed RPM_.  A _DNF History Frame_ is the output of an invocation of
//!    `dnf history info last-N` where `N` starts at OFFSET, and is incremented by one, at most,
//!    `MAX_DNF_DEJA_VU` times.
//!
//! 8. The collection of _DNF History Frames_ is then printed to the console for the User to inspect
//!    and decide if the changes are legitimate/expected, or not.
//!
//! 9. For each _changed RPM_ a yes/no prompt is put to the User to update `rkhunter` properties
//!    for that single RPM.
//!
//! 10. Once all _changed RPMs_ are processed, the tool prints elapsed time and exits.
//!
//!
//! How can you configure it?
//! ==
//! Make a copy of `.env.template`, rename it to **`rkh-chk.env`**, edit it to change the values
//! to your liking, and place it in the same directory where you'll be calling the binary from. If
//! you're interested in knowing more about how, when, and where from, environment variables are
//! loaded consult [dotenvy](https://docs.rs/dotenvy/latest/dotenvy/) and
//! [env-logger](https://docs.rs/env_logger/latest/env_logger/) documentation.
//! 
//! The comments in `.env.template`, and [`MyConfig`][1] hopefully provide all the needed information
//! about the configuration parameters.
//! 
//! 
//! Configuring when using the RPM
//! ==
//! Starting from version 1.0.1, this software offers a pre-packaged RPM available for download from
//! its GitHub repository.  Alternatively you can build the RPM yourself using the included script:
//! `build-rpm.sh`. After installing the RPM, the following two files will be added:
//! 
//! * `/usr/local/bin/rkh-chk` - the binary itself.
//! * `/usr/local/bin/rkh-chk.env.template` - the environment variables template.
//! 
//! Copy + rename the `.template` file as described earlier.  
//!
//!
//! [1]: crate::config::MyConfig
//! 

pub mod config;
pub mod dnf;
pub mod error;
pub mod rpm;
pub mod utils;
