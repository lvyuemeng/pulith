// Todo: alias command
// script
// template
// dispatch execution

use std::str::FromStr;

use thiserror::Error;

#[derive(Debug, Clone)]
pub struct CommandTemplate(Vec<TemplatePart>);

#[derive(Debug, Clone)]
enum TemplatePart {
    Literal(String),
    Positional(usize),
    Flag {
		// name: --flag
        name: String,
        alias: Vec<String>,
        multiple: bool,
    },
}

#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("Invalid placeholder syntax: {0}")]
    InvalidPlaceholder(String),
    #[error("Missing positional argument at index {0}")]
    MissingPositional(usize),
    #[error("Invalid flag sepecification: {0}")]
    InvalidFlag(String),
}

impl CommandTemplate {
    pub fn build_command(&self, name: &str) -> clap::Command {
        let mut cmd = clap::Command::new(name.to_string());

        for part in &self.0 {
            match part {
                TemplatePart::Literal(_) => {}
                TemplatePart::Positional(idx) => {
                    cmd = cmd.arg(clap::Arg::new(format!("pos_{}", idx)).index(*idx));
                }
                TemplatePart::Flag {
                    name,
                    alias,
                    multiple,
                } => {
					let name = name.trim_start_matches('-').to_string();
                    let long = name.clone();
                    let arg = clap::Arg::new(name).long(long).action(if *multiple {
                        clap::ArgAction::Append
                    } else {
                        clap::ArgAction::Set
                    });
                    let arg = alias.iter().fold(arg, |a, alias| {
                        if alias.starts_with('-') && alias.len() == 2 {
                            a.short(alias.chars().nth(1).unwrap())
                        } else {
                            a.alias(alias)
                        }
                    });
                    cmd = cmd.arg(arg);
                }
            }
        }
        cmd
    }
    pub fn expand(&self, matches: &clap::ArgMatches) -> Result<Vec<String>, TemplateError> {
        let mut args = vec![];

        for part in &self.0 {
            match part {
                TemplatePart::Literal(t) => args.push(t.clone()),
                TemplatePart::Positional(idx) => {
                    let val = matches
                        .get_one::<String>(&format!("pos_{}", idx))
                        .ok_or(TemplateError::MissingPositional(*idx))?;
                    args.push(val.clone());
                }
                TemplatePart::Flag {
                    name,
                    alias: _,
                    multiple,
                } => {
                    let name = name.clone();
                    if let Some(vals) = matches.get_many::<String>(&name) {
                        if *multiple {
                            args.push(name);
                            vals.into_iter().for_each(|val| args.push(val.to_string()));
                        } else if let Some(val) = vals.last() {
                            args.push(name);
                            args.push(val.clone());
                        }
                    }
                }
            }
        }
        Ok(args)
    }
    pub fn parse(tpl: &str) -> Result<Self, TemplateError> {
        let mut parts = Vec::new();
        let mut cur = String::new();
        // in_placeholder
        let mut in_ph = false;
        // placeholder
        let mut ph = String::new();

        for c in tpl.chars() {
            match (c, in_ph) {
                ('{', false) => {
                    if !cur.is_empty() {
						let left = cur.drain(..).collect();
                        parts.push(TemplatePart::Literal(left));
                    }
                    in_ph = true;
                }
                ('}', true) => {
                    parts.push(Self::parse_placeholder(&ph)?);
                    ph.clear();
                    in_ph = false;
                }
                (_, true) => ph.push(c),
                (_, false) => cur.push(c),
            }
        }

        if !cur.is_empty() {
            parts.push(TemplatePart::Literal(cur));
        }

        Ok(Self(parts))
    }

    fn parse_placeholder(s: &str) -> Result<TemplatePart, TemplateError> {
        let content = s.trim();

        // positional argument
        if let Ok(index) = content.parse::<usize>() {
            return Ok(TemplatePart::Positional(index));
        }

        // flag argument
        let mut parts = content.split_whitespace();
        let first = parts.next().ok_or(TemplateError::InvalidPlaceholder(
            "Empty flag placeholder".to_string(),
        ))?;

        if !first.starts_with("--") && !first.starts_with("-") {
            return Err(TemplateError::InvalidFlag(format!(
                "Invalid flag prefix {}",
                first
            )));
        }

        let mut alias = vec![];
        let mut multiple = false;
        for part in parts {
            if part == "*" {
                multiple = true;
            } else {
                alias.push(part.to_string());
            }
        }

        Ok(TemplatePart::Flag {
            name: first.to_string(),
            alias,
            multiple,
        })
    }
}

impl FromStr for CommandTemplate {
    type Err = TemplateError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}
