use tabled::{
    Table,
    settings::{Panel, Remove, Style, object::Rows},
    Tabled,
};

pub struct Formatter;

#[derive(Debug, Clone, Default)]
pub struct FormatConfig {
    pub header: Option<String>,
    pub footer: Option<String>,
    pub col_name: bool,
}

impl Formatter {
    pub fn default<T: Tabled, I: IntoIterator<Item = T>>(data: I, config: FormatConfig) -> Table {
        let mut table = Table::new(data);
        if let Some(header) = config.header {
            table.with(Panel::header(header));
        }
        if let Some(footer) = config.footer {
            table.with(Panel::footer(footer));
        }

        if config.col_name {
            table.with(Remove::row(Rows::first()));
        }

        table.with(Style::blank());
        table
    }
}
