use horizon_mod_sdk::storage;

#[derive(Clone, Debug)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
}

impl Value {
    fn write(&self) -> String {
        match self {
            Value::Bool(b) => if *b { "1".into() } else { "0".into() },
            Value::Int(i) => i.to_string(),
            Value::Float(f) => format!("{f:.2}"),
            Value::Text(t) => t.replace('\n', " "),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub key: String,
    pub value: Value,
    pub default: Value,
    pub min: f64,
    pub max: f64,
}

#[derive(Clone, Debug)]
pub struct Config {
    file: String,
    entries: Vec<Entry>,
}

impl Config {
    pub fn new(file: &str) -> Self {
        Self { file: file.to_owned(), entries: Vec::new() }
    }

    fn raw(&self) -> Vec<(String, String)> {
        storage::get(&self.file)
            .and_then(|v| String::from_utf8(v).ok())
            .map(|t| {
                t.lines()
                    .filter_map(|l| {
                        let l = l.trim();
                        if l.is_empty() || l.starts_with('#') {
                            return None;
                        }
                        let (k, v) = l.split_once('=')?;
                        Some((k.trim().to_owned(), v.trim().to_owned()))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn add(mut self, key: &str, default: Value, min: f64, max: f64) -> Self {
        self.entries.push(Entry { key: key.to_owned(), value: default.clone(), default, min, max });
        self
    }

    pub fn bool(self, key: &str, default: bool) -> Self {
        self.add(key, Value::Bool(default), 0.0, 1.0)
    }

    pub fn int(self, key: &str, default: i64, min: i64, max: i64) -> Self {
        self.add(key, Value::Int(default), min as f64, max as f64)
    }

    pub fn float(self, key: &str, default: f64, min: f64, max: f64) -> Self {
        self.add(key, Value::Float(default), min, max)
    }

    pub fn text(self, key: &str, default: &str) -> Self {
        self.add(key, Value::Text(default.to_owned()), 0.0, 0.0)
    }

    pub fn load(mut self) -> Self {
        let raw = self.raw();
        for e in &mut self.entries {
            let Some(v) = raw.iter().rev().find(|(k, _)| *k == e.key).map(|(_, v)| v.as_str()) else { continue };
            e.value = match &e.default {
                Value::Bool(_) => Value::Bool(matches!(v, "1" | "true" | "on" | "yes")),
                Value::Int(d) => Value::Int(v.parse::<i64>().map(|x| x.clamp(e.min as i64, e.max as i64)).unwrap_or(*d)),
                Value::Float(d) => Value::Float(v.parse::<f64>().ok().filter(|x| x.is_finite()).map(|x| x.clamp(e.min, e.max)).unwrap_or(*d)),
                Value::Text(_) => Value::Text(v.to_owned()),
            };
        }
        self
    }

    pub fn save(&self) -> Result<(), String> {
        let text: String = self.entries.iter().map(|e| format!("{}={}\n", e.key, e.value.write())).collect();
        storage::set(&self.file, text.as_bytes())
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    fn find(&self, key: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.key == key)
    }

    pub fn get_bool(&self, key: &str) -> bool {
        matches!(self.find(key).map(|e| &e.value), Some(Value::Bool(true)))
    }

    pub fn get_int(&self, key: &str) -> i64 {
        match self.find(key).map(|e| &e.value) {
            Some(Value::Int(i)) => *i,
            Some(Value::Float(f)) => *f as i64,
            _ => 0,
        }
    }

    pub fn get_float(&self, key: &str) -> f64 {
        match self.find(key).map(|e| &e.value) {
            Some(Value::Float(f)) => *f,
            Some(Value::Int(i)) => *i as f64,
            _ => 0.0,
        }
    }

    pub fn get_text(&self, key: &str) -> String {
        match self.find(key).map(|e| &e.value) {
            Some(Value::Text(t)) => t.clone(),
            Some(v) => v.write(),
            None => String::new(),
        }
    }

    pub fn set(&mut self, key: &str, value: Value) {
        if let Some(e) = self.entries.iter_mut().find(|e| e.key == key) {
            e.value = match value {
                Value::Int(i) => Value::Int(i.clamp(e.min as i64, e.max as i64)),
                Value::Float(f) if f.is_finite() => Value::Float(f.clamp(e.min, e.max)),
                Value::Float(_) => e.default.clone(),
                other => other,
            };
        }
    }

    pub fn reset(&mut self) {
        for e in &mut self.entries {
            e.value = e.default.clone();
        }
    }
}
