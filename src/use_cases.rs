use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::{cmp, fs};

use chrono::NaiveDate;

use crate::config;
use crate::log::{Day, Log};
use crate::model::{DaysAgo, Person};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LastInteraction {
    pub person: Person,
    pub last: NaiveDate,
}

impl LastInteraction {
    pub fn ago(self: &LastInteraction, reference: NaiveDate) -> DaysAgo {
        (reference - self.last).num_days()
    }
}

/// Get each person's last interaction
pub fn get_last_interactions(log: &Log) -> Vec<LastInteraction> {
    let mut last_interactions: HashMap<Person, NaiveDate> = HashMap::new();

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
        .map(|(person, date)| LastInteraction { person, last: date })
        .collect();

    interactions.sort_by_key(|interaction| (interaction.last, interaction.person.clone()));

    interactions
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

pub fn split_log_per_person(log: Log, config: &config::Config) -> HashMap<Person, Option<Log>> {
    let should_ignore: HashSet<Person> = config.ignore.clone().into_iter().collect();
    let mut per_person: HashMap<Person, Option<Log>> = HashMap::new();
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

fn infer_log_path(person: Person, dir: &PathBuf) -> PathBuf {
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

pub fn write_person_log(person: Person, log_opt: Option<Log>, dir: PathBuf) -> LogWritten {
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
            },
            LastInteraction {
                person: "JaneDoe".to_string(),
                last: d("2000-01-02"),
            },
            LastInteraction {
                person: "Abu".to_string(),
                last: d("2000-01-02"),
            },
        ];

        assert_eq!(sort_to_compare(summary), sort_to_compare(expected));
    }
}
