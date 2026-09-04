// SPDX-License-Identifier: GPL-3.0-or-later

//! Artifacts and functions related to the [DNF software package manager][1]
//! used in this tool's context.  For more details about specific use, consult
//! its [documentation][2].
//!
//! [1]: https://en.wikipedia.org/wiki/DNF_(software)
//! [2]: https://docs.fedoraproject.org/en-US/quick-docs/dnf/
//!

use crate::{config::config, error::MyError, utils::run_cmd_lenient};
use core::fmt;
use jiff::{Zoned, civil::DateTime};
use log::{debug, error, info};
use std::str::FromStr;

/// Summary of DNF transactions gleaned from the output of a `dnf history info`
/// call.  We also call this a _DNF History Frame_.
#[derive(Debug)]
pub struct DnfHistory {
    /// The ID of the DNF transaction that generated this history record.
    pub transaction_id: usize,
    /// The description which usually is the invoked command proper.
    pub description: String,
    /// Human readable transaction start time but represented in either
    /// system's timezone, or if unknown, user-preferred one if provided.
    /// Otherwise it's in UTC.
    pub begin_time: Zoned,
    /// Collection of [DnfEvent] describing what was done during this DNF
    /// call.
    pub events: Vec<DnfEvent>,
}

impl fmt::Display for DnfHistory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Transaction: #{}", self.transaction_id)?;
        write!(f, "When       : {}", self.begin_time)?;
        write!(f, "Description: {}", self.description)?;
        write!(f, "Events:")?;
        write!(
            f,
            "  Action     RPM                                           Reason               Repository"
        )?;
        write!(
            f,
            "  ---------  --------------------------------------------  -------------------  -----------"
        )?;
        for evt in &self.events {
            write!(f, "{}", evt)?
        }
        Ok(())
    }
}

impl DnfHistory {
    /// Given the output of a successful `dnf history info xxx` call, this
    /// parses the lines and construct a valid instance.
    pub fn try_from_cmd_output(lines: Vec<String>) -> Result<Self, MyError> {
        const TRANSACTION_ID: &str = "Transaction ID : ";
        const BEGIN_TIME: &str = "Begin time     : ";
        const DESCRIPTION: &str = "Description    : ";

        // must be at least 13 lines...
        if lines.len() < 13 {
            return Err(MyError::Runtime(
                "DNF History Info output MUST be at least 13 lines long".to_owned(),
            ));
        }
        let mut builder = DnfHistoryBuilder::default();
        for (ndx, line) in lines.iter().enumerate() {
            // always trim lines...
            let line = line.trim();
            match ndx {
                0 => {
                    // Transaction ID : 829
                    if !line.starts_with(TRANSACTION_ID) {
                        let msg = format!("DNF History Info line #{} is NOT Transaction ID", ndx);
                        return Err(MyError::Runtime(msg.to_owned()));
                    };
                    let id = line
                        .strip_prefix(TRANSACTION_ID)
                        .unwrap()
                        .parse::<usize>()?;
                    builder = builder.with_transaction_id(id);
                }
                1 => {
                    // Begin time
                    if !line.starts_with(BEGIN_TIME) {
                        let msg = format!("DNF History Info line #{} is NOT Begin time", ndx);
                        return Err(MyError::Runtime(msg.to_owned()));
                    };

                    let time = DateTime::from_str(line.strip_prefix(BEGIN_TIME).unwrap())?;
                    builder = builder.with_begin_time(time);
                }
                8 => {
                    // Description
                    if !line.starts_with(DESCRIPTION) {
                        let msg = format!("DNF History Info line #{} is NOT Description", ndx);
                        return Err(MyError::Runtime(msg.to_owned()));
                    };
                    let description = line.strip_prefix(DESCRIPTION).unwrap();
                    builder = builder.with_description(description);
                }
                // lines #12 and beyond are DNF Events...
                _x if ndx > 11 && !line.is_empty() => {
                    // Package altered lines...
                    let evt = DnfEvent::try_from(line)?;
                    builder = builder.add_event(evt);
                }
                _ => {} // ignored
            }
        }

        builder.build()
    }

    /// Return TRUE if designated `rpm` is mentioned in at least one of the DNF
    /// events of this frame.  Return FALSE otherwise.
    pub fn contains_rpm(&self, rpm: &str) -> bool {
        debug!("DnfHistory.contains_rpm(..., {})", rpm);
        for evt in &self.events {
            if evt.rpm.starts_with(rpm) {
                return true;
            }
        }
        false
    }

    /// Print general info about this specific DNF History frame, but skip
    /// events that are not pertaining to the designated RPMs.
    fn print_details(&self, rpms: &[String]) {
        // collect DnfEvents that mention any of given RPMs...
        let filtered: Vec<&DnfEvent> = self
            .events
            .iter()
            .filter(|x| rpms.iter().any(|y| x.rpm.starts_with(y)))
            .collect();
        if !filtered.is_empty() {
            info!("Transaction: #{}", self.transaction_id);
            info!("When       : {}", self.begin_time);
            info!("Description: {}", self.description);
            info!("Events:");
            info!(
                "  Action     RPM                                           Reason               Repository"
            );
            info!(
                "  ---------  --------------------------------------------  -------------------  -----------"
            );
            for evt in filtered {
                info!("  {}", evt)
            }
        }
    }
}

#[derive(Debug, Default)]
struct DnfHistoryBuilder {
    _transaction_id: Option<usize>,
    _description: Option<String>,
    _begin_time: Option<Zoned>,
    _events: Vec<DnfEvent>,
}

impl DnfHistoryBuilder {
    fn with_transaction_id(mut self, id: usize) -> Self {
        self._transaction_id = Some(id);
        self
    }

    fn with_description(mut self, description: &str) -> Self {
        self._description = Some(description.to_owned());
        self
    }

    fn with_begin_time(mut self, begin_time: DateTime) -> Self {
        let msg = format!("Failed setting begin time ({}) to UTC TZ", begin_time);
        let begin_time_utc = begin_time.in_tz("etc/utc").expect(&msg);
        let result = begin_time_utc.in_tz(&config().timezone).expect("msg");
        self._begin_time = Some(result);
        self
    }

    fn add_event(mut self, dnf_event: DnfEvent) -> Self {
        self._events.push(dnf_event);
        self
    }

    fn is_valid(&self) -> bool {
        self._transaction_id.is_some()
            && self._begin_time.is_some()
            && self._description.is_some()
            && !self._events.is_empty()
    }

    fn build(&self) -> Result<DnfHistory, MyError> {
        if !self.is_valid() {
            error!("Failed building this DNF History: {:?}", self);
            Err(MyError::Runtime("Invalid DNF History builder".to_owned()))
        } else {
            Ok(DnfHistory {
                transaction_id: self._transaction_id.unwrap(),
                begin_time: self._begin_time.as_ref().unwrap().to_owned(),
                description: self._description.as_ref().unwrap().to_owned(),
                events: self._events.to_owned(),
            })
        }
    }
}

/// Structure representing one line from the _Packages altered_ section of
/// a `dnf history info ...` command.
#[derive(Clone, Debug)]
pub struct DnfEvent {
    /// The DNF Action that is responsible for the event.
    pub action: DnfAction,
    /// The full name of the RPM package involved.
    pub rpm: String,
    /// The DNF Reason (essentially the command) that created this event.
    pub reason: DnfReason,
    /// The DNF Repository where the RPM comes from.
    pub repository: String,
}

impl fmt::Display for DnfEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}  {: <44.44}  {}  {}",
            self.action, self.rpm, self.reason, self.repository
        )
    }
}

impl TryFrom<&str> for DnfEvent {
    type Error = MyError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut iter = value.split_whitespace();
        let mut builder = DnfEventBuilder::default();
        builder = builder.try_with_action(iter.next().unwrap())?;
        builder = builder.with_rpm(iter.next().unwrap());
        // DNF Reasons can be 2 words...
        let word1 = iter.next().unwrap();
        let reason = match word1 {
            "Dependency" => DnfReason::Dependency,
            "Group" => DnfReason::Group,
            "User" => DnfReason::User,
            "Weak" => {
                let word2 = iter.next().unwrap();
                if word2 == "Dependency" {
                    DnfReason::WeakDependency
                } else {
                    return Err(MyError::Runtime("Unknown Weak xxx DNF Reason".to_owned()));
                }
            }
            "External" => {
                let word2 = iter.next().unwrap();
                if word2 == "User" {
                    DnfReason::ExternalUser
                } else {
                    return Err(MyError::Runtime(
                        "Unknown External xxx DNF Reason".to_owned(),
                    ));
                }
            }
            x => {
                let msg = format!("Unknown DN Readon: {}", x);
                return Err(MyError::Runtime(msg));
            }
        };
        builder = builder.with_reason(reason);
        builder = builder.with_repository(iter.next().unwrap());

        builder.build()
    }
}

#[derive(Debug, Default)]
struct DnfEventBuilder {
    _action: Option<DnfAction>,
    _rpm: Option<String>,
    _reason: Option<DnfReason>,
    _repository: Option<String>,
}

impl DnfEventBuilder {
    fn try_with_action(mut self, s: &str) -> Result<Self, MyError> {
        let action = DnfAction::try_from(s)?;
        self._action = Some(action);
        Ok(self)
    }

    fn with_rpm(mut self, s: &str) -> Self {
        self._rpm = Some(s.to_owned());
        self
    }

    fn with_reason(mut self, reason: DnfReason) -> Self {
        self._reason = Some(reason);
        self
    }

    fn with_repository(mut self, s: &str) -> Self {
        self._repository = Some(s.to_owned());
        self
    }

    fn is_valid(&self) -> bool {
        self._action.is_some()
            && self._rpm.is_some()
            && self._reason.is_some()
            && self._repository.is_some()
    }

    fn build(&self) -> Result<DnfEvent, MyError> {
        if !self.is_valid() {
            error!("Failed building a DNF Event: {:?}", self);
            Err(MyError::Runtime("Invalid DNF Event builder".to_owned()))
        } else {
            Ok(DnfEvent {
                action: self._action.as_ref().unwrap().to_owned(),
                rpm: self._rpm.as_ref().unwrap().to_owned(),
                reason: self._reason.as_ref().unwrap().to_owned(),
                repository: self._repository.as_ref().unwrap().to_owned(),
            })
        }
    }
}

/// Enumeration of DNF Actions that appear in a [DnfEvent]. More info is
/// available [here](https://dnf.readthedocs.io/en/latest/command_ref.html#history-command-label).
#[derive(Clone, Debug)]
pub enum DnfAction {
    /// A new package was installed.
    Install,
    /// A newer version of a package replaced a previously installed one.
    Upgrade,
    /// A previously installed package was removed.
    Remove,
    /// A new package was installed replacing an obsolete one.
    Replaced,
}

impl fmt::Display for DnfAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DnfAction::Install => write!(f, "{:<9}", "Install"),
            DnfAction::Upgrade => write!(f, "{:<9}", "Upgrade"),
            DnfAction::Remove => write!(f, "{:<9}", "Remove"),
            DnfAction::Replaced => write!(f, "{:<9}", "Replace"),
        }
    }
}

impl TryFrom<&str> for DnfAction {
    type Error = MyError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.trim() {
            "Install" => Ok(DnfAction::Install),
            "Upgrade" => Ok(DnfAction::Upgrade),
            "Remove" => Ok(DnfAction::Remove),
            "Replaced" => Ok(DnfAction::Replaced),
            x => {
                let msg = format!("Unknown DNF Action: {}", x);
                Err(MyError::Runtime(msg))
            }
        }
    }
}

/// Enumeration of DNF Reasons that appear in a [DnfEvent].
#[derive(Clone, Debug)]
pub enum DnfReason {
    /// A required dependency of another packge.
    Dependency,
    /// A weak dependency pulled by another package.
    WeakDependency,
    /// Package installed as part of a group.
    Group,
    /// Package requested by a user.
    User,
    /// Package w/ lost or unknown metadata.
    ExternalUser,
}

impl fmt::Display for DnfReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DnfReason::Dependency => write!(f, "{:<19}", "Dependency"),
            DnfReason::WeakDependency => write!(f, "{:<19}", "Weak Dependency"),
            DnfReason::Group => write!(f, "{:<19}", "Group"),
            DnfReason::User => write!(f, "{:<19}", "User"),
            DnfReason::ExternalUser => write!(f, "{:<19}", "External User"),
        }
    }
}

impl TryFrom<&str> for DnfReason {
    type Error = MyError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.trim() {
            "Dependency" => Ok(Self::Dependency),
            "Weak Dependency" => Ok(Self::WeakDependency),
            "Group" => Ok(Self::Group),
            "User" => Ok(Self::User),
            "External User" => Ok(Self::ExternalUser),
            x => {
                let msg = format!("Unknown DNF Reason: {}", x);
                Err(MyError::Runtime(msg))
            }
        }
    }
}

/// Structure representing a collection of `dnf history info` which
/// showed one or more DNF events pertaining to installed RPMs that
/// own one or more changed files.
#[derive(Debug)]
pub struct DnfHistoryFrames {
    pub(crate) frames: Vec<DnfHistory>,
}

impl Default for DnfHistoryFrames {
    fn default() -> Self {
        Self {
            frames: Vec::with_capacity(config().max_dnf_deja_vu + 1),
        }
    }
}

impl DnfHistoryFrames {
    /// Add a DNF History frame/record...
    pub fn grow(mut self, offset: usize) -> Result<Self, MyError> {
        let len = self.frames.len();
        if len > config().max_dnf_deja_vu {
            return Err(MyError::Runtime(
                "Maximum DNF History frames count reached".to_owned(),
            ));
        }
        let ndx = format!("last-{}", offset + len);
        let x = dnf_history_info(&ndx)?;
        self.frames.push(x);
        Ok(self)
    }

    /// Return the size of this collection.
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Return whther this collection is empty (TRUE) or not (FALSE).
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Return TRUE if given RPM is mentioned in any DNF Events w/in the
    /// frame at given index. Return FALSE otherwise.
    pub fn contains_rpm(&self, rpm: &str, ndx: usize) -> bool {
        let result = self.frames[ndx].contains_rpm(rpm);
        debug!(
            "DnfHistoryFrames.contains_rpm(..., {}, {})? {}",
            rpm, ndx, result
        );
        result
    }

    /// Output DNF history records pertaining to given list of RPMs.
    pub fn print_details(&self, rpms: &[String]) {
        info!("----- DNF History Info Record(s) -----");
        for frame in &self.frames {
            frame.print_details(rpms);
        }
        info!("-----")
    }
}

/// invoke `dnf history info ndx`, parse the result and return it.
fn dnf_history_info(ndx: &str) -> Result<DnfHistory, MyError> {
    match run_cmd_lenient("dnf", &["history", "info", ndx]) {
        Ok(x) => {
            let result = DnfHistory::try_from_cmd_output(x)?;
            Ok(result)
        }
        Err(x) => Err(x),
    }
}
