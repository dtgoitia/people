use std::collections::HashSet;
use std::process;

use people::config;
use people::log;
use people::model::DaysAgo;
use people::model::Person;
use people::use_cases;
use people::use_cases::LastInteraction;
use tracing::info;

use chrono::Local;
use tabular::{Row, Table};

fn discard_ignored(
    interactions: Vec<LastInteraction>,
    config: &config::Config,
) -> Vec<LastInteraction> {
    let mut ignored: HashSet<Person> = HashSet::new();
    for person in &config.ignore {
        ignored.insert(person.clone());
    }

    interactions
        .into_iter()
        .filter(|interaction| ignored.contains(&interaction.person) == false)
        .collect()
}

type BoundaryOffset = usize;
type Boundary = i64;

struct Spacer {
    boundaries: Vec<Boundary>,
    next_boundary_offset: Option<BoundaryOffset>,
}

impl Spacer {
    fn new(boundaries: Vec<Boundary>) -> Spacer {
        let no_boundaries = *(&boundaries.is_empty());

        Spacer {
            boundaries,
            next_boundary_offset: if no_boundaries { None } else { Some(0) },
        }
    }

    fn next_boundary(&self) -> Option<Boundary> {
        match self.next_boundary_offset {
            Some(offset) => {
                if let Some(boundary_ref) = self.boundaries.get(offset) {
                    return Some(boundary_ref.clone());
                } else {
                    return None;
                }
            }
            None => return None,
        }
    }

    fn jump_to_next_boundary(&mut self) -> () {
        let last_offset = match self.next_boundary_offset {
            Some(offset) => offset,
            None => {
                self.next_boundary_offset = None;
                return;
            }
        };

        let new_offset = last_offset + 1;

        if new_offset < self.boundaries.len() {
            self.next_boundary_offset = Some(new_offset);
        } else {
            self.next_boundary_offset = None;
        }
    }

    fn should_show_space(&mut self, current: DaysAgo) -> bool {
        let next_boundary = match self.next_boundary() {
            None => return false,
            Some(boundary) => boundary,
        };

        let show_space = current >= next_boundary;

        if show_space {
            self.jump_to_next_boundary();
        }

        show_space
    }
}

fn format_last_interactions(interactions: Vec<LastInteraction>) -> String {
    let today = Local::now().naive_local().date();

    let mut sorted_interactions = interactions.clone();
    sorted_interactions.sort_by_key(|interaction| interaction.last);
    sorted_interactions.reverse();

    let mut table = Table::new("{:>}  {:<}  {:<}");
    table.add_row(
        Row::new()
            .with_cell("Days ago")
            .with_cell("PERSON")
            .with_cell("LAST"),
    );

    let empty_row = Row::new().with_cell("").with_cell("").with_cell("");

    let mut spacer = Spacer::new(vec![7, 14, 28]);

    for interaction in sorted_interactions {
        let ago = interaction.ago(today);
        if spacer.should_show_space(ago) {
            table.add_row(empty_row.clone());
        }

        table.add_row(
            Row::new()
                .with_cell(ago)
                .with_cell(interaction.person)
                .with_cell(interaction.last),
        );
    }

    format!("{table}")
}

fn main() {
    info!("Loading config...");
    let config = match config::get_config() {
        Ok(config) => config,
        Err(reason) => {
            eprintln!("ERROR: {}", reason);
            process::exit(1);
        }
    };

    let log = log::read_logs(&config.people_dir);

    let all = use_cases::get_last_interactions(&log);
    let desired = discard_ignored(all, &config);
    let summary = format_last_interactions(desired);
    println!("{summary}");
}
