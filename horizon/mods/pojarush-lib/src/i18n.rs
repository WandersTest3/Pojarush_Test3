use horizon_mod_sdk::ui;

pub struct Lang<const N: usize> {
    pub table: &'static [(&'static str, [&'static str; N])],
}

impl<const N: usize> Lang<N> {
    pub const fn new(table: &'static [(&'static str, [&'static str; N])]) -> Self {
        Self { table }
    }

    pub fn strings(&self) -> &'static [&'static str; N] {
        let code = ui::language().to_lowercase();
        let base = code.split('_').next().unwrap_or("").to_owned();
        self.table
            .iter()
            .find(|(c, _)| *c == code)
            .or_else(|| self.table.iter().find(|(c, _)| *c == base || c.split('_').next() == Some(base.as_str())))
            .map(|(_, s)| s)
            .unwrap_or(&self.table[0].1)
    }

    pub fn get(&self, index: usize) -> &'static str {
        self.strings().get(index).copied().unwrap_or("")
    }
}
