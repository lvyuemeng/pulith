use tabled::{
    Table,
    settings::{Panel, Remove, Style, object::Rows},
    Tabled,
};

#[derive(Debug, Clone, Default)]
pub struct Formatter {
    pub header: Option<String>,
    pub footer: Option<String>,
    pub col_name: bool,
}

impl Formatter {
    pub fn build<T: Tabled, I: IntoIterator<Item = T>>(self,data: I) -> Table {
        let mut table = Table::new(data);
        if let Some(header) = self.header {
            table.with(Panel::header(header));
        }
        if let Some(footer) = self.footer {
            table.with(Panel::footer(footer));
        }

        if self.col_name {
            table.with(Remove::row(Rows::first()));
        }

        table.with(Style::blank());
        table
    }
}
