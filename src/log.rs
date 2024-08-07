use core::fmt;
use glob::glob;
use regex::Regex;
use std::{collections::HashSet, fs, path::PathBuf};
use textwrap::dedent;

use crate::model::PersonName;
use chrono::NaiveDate;

static TAB: &str = "	";
static TWO_SPACES: &str = "  ";

type EntryContent = String;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub main: HashSet<PersonName>,
    pub related: HashSet<PersonName>,
    pub content: EntryContent,
}

impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.content)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Day {
    pub date: NaiveDate,
    pub entries: Vec<Entry>,
}

impl fmt::Display for Day {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let date = self.date;
        let entries: Vec<String> = self.entries.iter().map(|entry| entry.to_string()).collect();
        let fmt_entries = entries.join("\n");
        let content = format!("# {date}\n\n{fmt_entries}");
        write!(f, "{content}")
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Log {
    pub days: Vec<Day>,
}

impl fmt::Display for Log {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let days: Vec<String> = self.days.iter().map(|day| day.to_string()).collect();
        let content = days.join("\n\n");
        write!(f, "{content}\n")
    }
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
    let line_no_tabs = line.replace(TAB, TWO_SPACES);
    let indentation = find_first_non_space(&line_no_tabs);
    let content = &line_no_tabs[indentation..];

    let token = Token {
        line_number,
        indentation,
        content: content.to_string(),
    };

    token
}

fn tokenize(content: &str) -> Vec<Token> {
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

fn parse_people(token: &Token) -> HashSet<PersonName> {
    let pattern = r"\#([A-Za-zñáéíóúç]+)"; // TODO: make it constant
    let re = Regex::new(pattern).unwrap();

    let people_in_token: HashSet<PersonName> = re
        .captures_iter(&token.content)
        .map(|cap| cap[1].to_string())
        .collect();

    let mut people: HashSet<PersonName> = HashSet::new();
    people.extend(people_in_token);

    people
}

fn parse_entry(tokens: Vec<Token>) -> Entry {
    let first_token = &tokens[0];
    let main: HashSet<PersonName> = parse_people(first_token);

    let mut related: HashSet<PersonName> = HashSet::new();
    let mut content_lines: Vec<String> = vec![];

    for token in tokens {
        let people_in_token = parse_people(&token);
        related.extend(people_in_token);

        let indendation = " ".repeat(token.indentation);
        let content_line = vec![indendation, token.content].join("");
        content_lines.push(content_line);
    }

    Entry {
        main,
        related,
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

pub fn parse_log_file_content(content: &str) -> Log {
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

fn find_log_files(people_dir: &PathBuf) -> Vec<PathBuf> {
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

pub fn read_logs(people_dir: &PathBuf) -> Log {
    let mut days: Vec<Day> = vec![];

    let files = find_log_files(people_dir);
    for path in files {
        let content = fs::read_to_string(&path).unwrap();
        let log = parse_log_file_content(&content);
        days.extend(log.days);
    }

    Log { days }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use pretty_assertions::assert_eq;

    use crate::test_utils::d;

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
        let content = indoc!(
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
            "
        );

        let expected = Log {
            days: vec![
                Day {
                    date: d("2000-01-01"),
                    entries: vec![Entry {
                        main: ["JohnDoe".to_string()].into(),
                        related: ["JohnDoe".to_string()].into(),
                        content: "- #JohnDoe :\n  - stuff: blah".to_string(),
                    }],
                },
                Day {
                    date: d("2000-01-02"),
                    entries: vec![
                        Entry {
                            main: ["JohnDoe".to_string()].into(),
                            related: ["JohnDoe".to_string(), "Bleh".to_string()].into(),
                            content: "- #JohnDoe :\n  - stuff: blah\n  - other: bleh #Bleh"
                                .to_string(),
                        },
                        Entry {
                            main: ["JaneDoe".to_string(), "Abu".to_string()].into(),
                            related: ["JaneDoe".to_string(), "Abu".to_string()].into(),
                            content: "- #JaneDoe, #Abu :\n  - meet at foo\n    - nested stuff"
                                .to_string(),
                        },
                    ],
                },
            ],
        };

        assert_eq!(parse_log_file_content(&content), expected);
    }

    #[test]
    fn test_support_special_characters() {
        let content = indoc!(
            "
            # 2000-01-01

            - #Lucía:
              - stuff: blah
            ",
        );

        let expected = Log {
            days: vec![Day {
                date: d("2000-01-01"),
                entries: vec![Entry {
                    main: ["Lucía".to_string()].into(),
                    related: ["Lucía".to_string()].into(),
                    content: "- #Lucía:\n  - stuff: blah".to_string(),
                }],
            }],
        };

        assert_eq!(parse_log_file_content(&content), expected);
    }

    #[test]
    fn test_replace_tabs_with_two_spaces() {
        // NOTE: there is a tab immediately before `- stuff: blah`
        let content = indoc!(
            "
            # 2000-01-01

            - #Lucía:
            	- stuff: blah
            "
        );

        let expected = Log {
            days: vec![Day {
                date: d("2000-01-01"),
                entries: vec![Entry {
                    main: ["Lucía".to_string()].into(),
                    related: ["Lucía".to_string()].into(),
                    content: "- #Lucía:\n  - stuff: blah".to_string(),
                }],
            }],
        };

        assert_eq!(parse_log_file_content(&content), expected);
    }

    #[test]
    fn test_display_log() {
        let content = indoc!(
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
            "
        );

        let log = parse_log_file_content(&content);
        let formatted = format!("{log}");
        println!("\n{content:#?}");
        println!("\n{formatted:#?}");

        assert_eq!(formatted, content);
    }
}
