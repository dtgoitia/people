use glob::glob;
use regex::Regex;
use std::{collections::HashSet, fs, path::PathBuf};
use textwrap::dedent;

use crate::model::Person;
use chrono::NaiveDate;

type EntryContent = String;

#[derive(Debug, PartialEq, Eq)]
struct Entry {
    people: HashSet<Person>,
    content: EntryContent,
}

#[derive(Debug, PartialEq, Eq)]
struct Day {
    date: NaiveDate,
    entries: Vec<Entry>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Log {
    days: Vec<Day>,
}

#[derive(Debug, Clone)]
struct Token {
    line_number: usize,
    indentation: usize, // amount of spaces
    content: String,
}

impl Into<Line> for Token {
    fn into(self) -> Line {
        if token_is_empty_line(&self) {
            return Line::Empty;
        }

        if let Ok(date) = self.try_into_date() {
            return Line::Date(date);
        }

        return Line::Record(self);
    }
}

impl Token {
    fn try_into_date(self: &Token) -> Result<Date, ()> {
        if self.indentation != 0 {
            return Err(());
        }

        if !self.content.starts_with("# ") {
            return Err(());
        }

        let date_str = &self.content[2..].trim_end();

        let date = match NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            Ok(date) => date,
            Err(_) => return Err(()),
        };

        Ok(Date {
            line_number: self.line_number,
            value: date,
        })
    }
}

#[derive(Debug, Clone)]
struct Date {
    #[allow(dead_code)]
    line_number: usize,
    value: NaiveDate,
}

impl From<Line> for Date {
    fn from(line: Line) -> Date {
        match line {
            Line::Date(date) => return date,
            _ => panic!("you should have never reached this point"),
        }
    }
}

#[derive(Debug, Clone)]
enum Line {
    Empty,
    Date(Date),
    Record(Token),
}

fn find_first_non_space(input: &str) -> usize {
    input
        .char_indices()
        .find(|(_, ch)| !ch.is_whitespace())
        .map(|(i, _)| i)
        .unwrap_or(0)
}

fn tokenize_line(line: String, line_number: usize) -> Token {
    let indentation = find_first_non_space(&line);
    let content = &line[indentation..];

    let token = Token {
        line_number,
        indentation,
        content: content.to_string(),
    };

    token
}

fn tokenize(content: String) -> Vec<Token> {
    let lines = content.split("\n");

    let mut tokens: Vec<Token> = vec![];
    for (line_number, line) in lines.into_iter().enumerate() {
        let token = tokenize_line(line.to_string(), line_number);
        tokens.push(token);
    }

    tokens
}

fn token_is_empty_line(token: &Token) -> bool {
    token.indentation == 0 && token.content.is_empty()
}

fn parse_entry(tokens: Vec<Token>) -> Entry {
    let pattern = r"\#([A-Za-z]+)"; // TODO: make it constant
    let re = Regex::new(pattern).unwrap();

    let mut people: HashSet<Person> = HashSet::new();
    let mut content_lines: Vec<String> = vec![];
    for token in tokens {
        let people_in_token: HashSet<Person> = re
            .captures_iter(&token.content)
            .map(|cap| cap[1].to_string())
            .collect();
        people.extend(people_in_token);

        let indendation = " ".repeat(token.indentation);
        let content_line = vec![indendation, token.content].join("");
        content_lines.push(content_line);
    }

    Entry {
        people,
        content: dedent(&content_lines.join("\n")),
    }
}

fn parse_day(date: Date, lines: Vec<Token>) -> Day {
    let mut entries: Vec<Entry> = vec![];

    let mut buffer: Vec<Token> = vec![];

    for token in lines.iter() {
        let is_top_level = token.indentation == 0;
        if is_top_level && !buffer.is_empty() {
            entries.push(parse_entry(buffer));
            buffer = vec![];
        }
        buffer.push(token.clone());
    }

    if !buffer.is_empty() {
        entries.push(parse_entry(buffer));
    }

    Day {
        date: date.value,
        entries,
    }
}

#[allow(unused_assignments)]
fn parse_log_file_content(content: String) -> Log {
    let tokens = tokenize(content);

    let mut buffered_date: Option<Date> = None;
    let mut buffered_lines: Vec<Token> = vec![];
    let mut days: Vec<Day> = vec![];
    for token in tokens {
        match Into::<Line>::into(token) {
            Line::Empty => {} // skip
            Line::Date(date) => {
                if !buffered_lines.is_empty() {
                    let day = parse_day(
                        buffered_date.expect("expected some date when lines are buffered"),
                        buffered_lines,
                    );
                    days.push(day);

                    buffered_date = None;
                    buffered_lines = vec![];
                }
                buffered_date = Some(date);
            }
            Line::Record(token) => {
                buffered_lines.push(token);
            }
        }
    }

    if !buffered_lines.is_empty() {
        if buffered_date.is_none() {
            panic!("expected some date when lines are buffered")
        }
        let day = parse_day(buffered_date.unwrap(), buffered_lines);
        days.push(day);
    }

    Log { days }
}

fn find_log_files(people_dir: PathBuf) -> Vec<PathBuf> {
    let base = people_dir.to_string_lossy();
    let pattern = format!("{base}/log/*people.md");

    let mut files: Vec<PathBuf> = vec![];
    for entry in glob(&pattern).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => files.push(path),
            _ => (),
        }
    }

    files.sort();

    return files;
}

pub fn read_logs(people_dir: PathBuf) -> Log {
    let mut days: Vec<Day> = vec![];

    let files = find_log_files(people_dir);
    for path in files {
        let content = fs::read_to_string(&path).unwrap();
        let log = parse_log_file_content(content);
        days.extend(log.days);
    }

    Log { days }
}

#[cfg(test)]
mod tests {
    use textwrap::dedent;

    use super::*;

    #[test]
    fn test_find_first_non_space_character_when_string_is_not_indented() {
        assert_eq!(find_first_non_space("foo"), 0);
    }

    #[test]
    fn test_find_first_non_space_character_when_string_is_indented() {
        assert_eq!(find_first_non_space("  foo"), 2);
    }

    #[test]
    fn test_parse_log_file() {
        let content = dedent(
            r#"
            # 2000-01-01

            - #JohnDoe :
              - stuff: blah
              - other: bleh
            - #JaneDoe, #Abu :
              - meet at foo
                - nested stuff
            "#,
        )
        .to_string();

        let expected = Log {
            days: vec![Day {
                date: NaiveDate::from_ymd(2000, 1, 1),
                entries: vec![
                    Entry {
                        people: vec!["JohnDoe".to_string()].into_iter().collect(),
                        content: "- #JohnDoe :\n  - stuff: blah\n  - other: bleh".to_string(),
                    },
                    Entry {
                        people: vec!["JaneDoe".to_string(), "Abu".to_string()]
                            .into_iter()
                            .collect(),
                        content: "- #JaneDoe, #Abu :\n  - meet at foo\n    - nested stuff"
                            .to_string(),
                    },
                ],
            }],
        };

        assert_eq!(parse_log_file_content(content), expected);
    }
}
