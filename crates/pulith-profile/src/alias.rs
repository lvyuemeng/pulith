// Todo: alias command
// script
// template
// dispatch execution

use thiserror::Error;

enum Template {
    Literal(String),
    Positional(usize),
    Variadic,
    Flag {
        names: Vec<FlagName>,
        multiple: bool,
    },
}

#[derive(Debug, Error)]
enum TemplateError {
    InvalidPlaceholder,
    InvalidFlag,
}

enum FlagName {
    Short(char),
    Long(String),
}

impl Template {
    pub fn parse_template(template: &str) -> Result<Vec<Template>, TemplateError> {
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut in_placeholder = false;
        let mut placeholder_content = String::new();

        for c in template.chars() {
            match (c, in_placeholder) {
                ('{', false) => {
                    if !current.is_empty() {
                        parts.push(Template::Literal(current));
                        current.clear();
                    }
                    in_placeholder = true;
                }

                ('}', true) => {
                    let part = match parse_placeholder(&placeholder_content) {
                        Ok(p) => p,
                        Err(e) => Err(e),
                    };
                    parts.push(part);
                    placeholder_content.clear();
                    in_place = false;
                }
                (_, true) => placeholder_content.push(c),
                (_, false) => current.push(c),
            }
        }

        if !current.is_empty() {
            parts.push(Template::Literal(current));
        }
        Ok(parts)
    }

    pub fn to_command(name: &str, s: Vec<Template>) -> clap::Command {
        let command = clap::Command::new(name.to_string());
        s.iter().fold(command, |c, t| match t {
            Template::Literal(_) => c,
            Template::Positional(index) => c.arg(clap::Arg::new("pos{index}").index(*index)),
            Template::Variadic => c.arg(clap::Arg::new("variadic").action(clap::ArgAction::Append)),
            Template::Flag { names, multiple } => {
                let mut arg = clap::Arg::new("flag");
                for n in names {
                    match n {
                        FlagName::Short(n) => arg = arg.short(*n),
                        FlagName::Long(n) => arg = arg.long(n),
                    }
                }

                if *multiple {
                    arg = arg.action(clap::ArgAction::Append);
                }
                c.arg(arg)
            }
        })
    }
}

fn parse_placeholder(content: &str) -> Result<Template, TemplateError> {
    let mut content = content.trim();
    if content == "*" {
        return Ok(Template::Variadic);
    }

    if let Ok(index) = content.parse::<usize>() {
        return Ok(Template::Positional(index));
    }

    let names = content.split_whitespace().collect::<Vec<_>>();
    let mut res = Vec::new();
    let mut multiple = false;
    for n in names {
        if n.starts_with("-") {
            let n = {
                n.trim_start_matches("-");
                if n.len() == 1 {
                    FlagName::Short(n.chars().next().unwrap())
                } else {
                    FlagName::Long(n.to_string())
                }
            };
            res.push(n);
        }
        if n == "*" {
            multiple = true;
        }
    }
    if res.is_empty() {
        return Err(TemplateError::InvalidFlag);
    }

    let res = Template::Flag {
        names: res,
        multiple,
    };
    Ok(res)
}
