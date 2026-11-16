use horizon_mod_sdk::input;

pub struct Keys {
    names: Vec<String>,
    down: Vec<String>,
}

impl Default for Keys {
    fn default() -> Self {
        Self { names: Vec::new(), down: Vec::new() }
    }
}

impl Keys {
    pub fn register(&mut self, name: &str, label: &str, key: &str) -> Result<(), String> {
        input::register_key(name, label, key)?;
        self.names.push(name.to_owned());
        Ok(())
    }

    pub fn pressed(&mut self, name: &str) {
        if !self.down.iter().any(|n| n == name) {
            self.down.push(name.to_owned());
        }
    }

    pub fn released(&mut self, name: &str) {
        self.down.retain(|n| n != name);
    }

    pub fn is_down(&self, name: &str) -> bool {
        self.down.iter().any(|n| n == name)
    }

    pub fn registered(&self) -> &[String] {
        &self.names
    }
}
