use std::cmp;
use std::collections::HashMap;

use chrono::NaiveDate;

use crate::log::Log;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{log, test_utils::d};
    use pretty_assertions::assert_eq;
    use textwrap::dedent;

    fn sort_to_compare(summary: Vec<LastInteraction>) -> Vec<LastInteraction> {
        let mut copy = summary.clone();
        copy.sort_by_key(|interaction| (interaction.last, interaction.person.clone()));
        copy
    }

    #[test]
    fn test_get_last_interactions() {
        let log = log::parse_log_file_content(&dedent(
            r#"
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
            "#,
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
