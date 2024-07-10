use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::{cmp, fs};

use chrono::{Duration, Local, NaiveDate};

use crate::config::{self, Config};
use crate::log::{Day, Log};
use crate::model::{DaysAgo, PersonName};

const DAYS_IN_A_MONTH: i64 = 30;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LastInteraction {
    pub person: PersonName,
    pub last: NaiveDate,
    pub days_beyond_reachout_threshold: Option<DaysAgo>,
}

impl LastInteraction {
    pub fn ago(self: &LastInteraction, reference: NaiveDate) -> DaysAgo {
        (reference - self.last).num_days()
    }

    pub fn assess_reminder(self: &LastInteraction, reminder_after: Duration) -> LastInteraction {
        let today = Local::now().naive_local().date();

        let threshold = self.last + reminder_after;
        let time_to_threshold = threshold - today;
        let days_to_threshold = time_to_threshold.num_days();

        LastInteraction {
            person: self.person.clone(),
            last: self.last,
            days_beyond_reachout_threshold: if days_to_threshold >= 0 {
                None
            } else {
                Some(-days_to_threshold)
            },
        }
    }
}

/// Get each person's last interaction
pub fn get_last_interactions(log: &Log) -> Vec<LastInteraction> {
    let mut last_interactions: HashMap<PersonName, NaiveDate> = HashMap::new();

    for day in log.days.iter() {
        for entry in day.entries.iter() {
            for person in entry.main.iter() {
                let desired_date: NaiveDate;

                if let Some(existing_date) = last_interactions.get(person) {
                    desired_date = cmp::max(day.date, *existing_date);
                } else {
                    desired_date = day.date;
                }

                last_interactions.insert(person.clone(), desired_date);
            }
        }
    }

    let mut interactions: Vec<LastInteraction> = last_interactions
        .into_iter()
        .map(|(person, date)| LastInteraction {
            person,
            last: date,
            days_beyond_reachout_threshold: None,
        })
        .collect();

    interactions.sort_by_key(|interaction| (interaction.last, interaction.person.clone()));

    interactions
}

/// Identify who should have been reached out and how long ago
pub fn identify_reachouts(
    without_reminders: Vec<LastInteraction>,
    config: &Config,
) -> Result<Vec<LastInteraction>, String> {
    let mut to_be_reminded: HashMap<PersonName, Duration> = HashMap::new();
    for person in &config.people {
        if let Some(duration_str) = person.remind_after.clone() {
            let duration = match parse_duration_text(duration_str) {
                Ok(d) => d,
                Err(reason) => return Err(reason),
            };
            to_be_reminded.insert(person.name.clone(), duration);
        }
    }

    let mut with_reminder: Vec<LastInteraction> = vec![];

    for interaction in without_reminders {
        if let Some(reminder) = to_be_reminded.get(&interaction.person) {
            with_reminder.push(interaction.assess_reminder(*reminder));
        } else {
            with_reminder.push(interaction);
        }
    }

    Ok(with_reminder)
}

fn parse_duration_text(str: String) -> Result<Duration, String> {
    let parts: Vec<&str> = str.split_whitespace().collect();
    let amount_str = parts[0];
    let amount: i64 = match amount_str.parse() {
        Ok(amount) => amount,
        Err(_) => {
            return Err(format!(
                "failed to parse '{str}', reason: unsupported amount found: {amount_str:?}"
            ));
        }
    };

    let unit = parts[1];

    match unit {
        "month" | "months" => Ok(Duration::days(amount * DAYS_IN_A_MONTH)),
        "week" | "weeks" => Ok(Duration::weeks(amount)),
        "day" | "days" => Ok(Duration::days(amount)),
        _ => {
            return Err(format!(
                "failed to parse '{str}', reason: unsupported unit found: {unit:?}"
            ));
        }
    }
}

fn merge_days(previous: Day, new: Day) -> Day {
    Day {
        date: previous.date,
        entries: vec![previous.entries, new.entries].concat(),
    }
}

fn merge_logs(previous: Log, new: Log) -> Log {
    let mut previous_days: HashMap<NaiveDate, Day> = previous
        .days
        .into_iter()
        .map(|day| (day.date, day))
        .collect();

    let mut days: Vec<Day> = vec![];
    for day in new.days {
        let date = day.date;

        let merged_day: Day;
        if let Some(previous_day) = previous_days.remove(&date) {
            merged_day = merge_days(previous_day, day);
        } else {
            merged_day = day;
        }
        days.push(merged_day);
    }

    for previous_day in previous_days.into_values() {
        days.push(previous_day);
    }

    days.sort_by_key(|day| day.date);

    Log { days }
}

pub fn split_log_per_person(log: Log, config: &config::Config) -> HashMap<PersonName, Option<Log>> {
    let should_ignore: HashSet<PersonName> = config.ignore.clone().into_iter().collect();
    let mut per_person: HashMap<PersonName, Option<Log>> = HashMap::new();
    for day in log.days {
        for entry in day.entries {
            for person in entry.clone().related {
                if should_ignore.contains(&person) {
                    per_person.insert(person, None);
                    continue;
                }

                let new = Log {
                    days: vec![Day {
                        date: day.date,
                        entries: vec![entry.clone()],
                    }],
                };

                if let Some(existing_log) = per_person.remove(&person) {
                    if let Some(previous) = existing_log {
                        let updated = merge_logs(previous, new);
                        per_person.insert(person, Some(updated));
                    } else {
                        per_person.insert(person, Some(new));
                    }
                } else {
                    per_person.insert(person, Some(new));
                }
            }
        }
    }

    per_person
}

fn infer_log_path(person: PersonName, dir: &PathBuf) -> PathBuf {
    let file_name = format!("{person}.md");
    let path = dir.join(file_name);
    path
}

type ErrorReason = String;

pub enum LogWritten {
    Written(PathBuf),
    FailedToWrite(PathBuf, ErrorReason),
    NothingToDelete(PathBuf),
    Deleted(PathBuf),
    FailedToDelete(PathBuf, ErrorReason),
}

pub fn write_person_log(person: PersonName, log_opt: Option<Log>, dir: PathBuf) -> LogWritten {
    let path = infer_log_path(person, &dir);

    if let Some(log) = log_opt {
        let content = format!("{log}");
        match fs::write(path.clone(), content) {
            Ok(()) => LogWritten::Written(path),
            Err(reason) => LogWritten::FailedToWrite(path, format!("{reason}")),
        }
    } else {
        // delete logs of ignored people
        if path.exists() {
            match fs::remove_file(path.clone()) {
                Ok(()) => LogWritten::Deleted(path),
                Err(reason) => LogWritten::FailedToDelete(path, format!("{reason}")),
            }
        } else {
            LogWritten::NothingToDelete(path)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{log, test_utils::d};
    use indoc::indoc;
    use pretty_assertions::assert_eq;

    fn sort_to_compare(summary: Vec<LastInteraction>) -> Vec<LastInteraction> {
        let mut copy = summary.clone();
        copy.sort_by_key(|interaction| (interaction.last, interaction.person.clone()));
        copy
    }

    #[test]
    fn test_get_last_interactions() {
        let log = log::parse_log_file_content(indoc!(
            "
            # 2000-01-01

            - #JohnDoe :
              - stuff: blah

            # 2000-01-02

            - #JohnDoe :
              - stuff: blah
              - other: bleh #Bleh
            - #JaneDoe, #Abu :
              - meet at foo
                - nested stuff
            ",
        ));

        let summary = get_last_interactions(&log);

        let expected = vec![
            LastInteraction {
                person: "JohnDoe".to_string(),
                last: d("2000-01-02"),
                days_beyond_reachout_threshold: None,
            },
            LastInteraction {
                person: "JaneDoe".to_string(),
                last: d("2000-01-02"),
                days_beyond_reachout_threshold: None,
            },
            LastInteraction {
                person: "Abu".to_string(),
                last: d("2000-01-02"),
                days_beyond_reachout_threshold: None,
            },
        ];

        assert_eq!(sort_to_compare(summary), sort_to_compare(expected));
    }
}
