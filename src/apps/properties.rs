use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

pub struct Properties {
    props: HashMap<String, String>,
}

impl Properties {
    pub fn new(file_path: &str) -> io::Result<Self> {
        let path = Path::new(file_path);
        let file = File::open(&path)?;
        let reader = io::BufReader::new(file);

        let mut props = HashMap::new();

        for line in reader.lines() {
            let line = line?;
            let mut parts = line.splitn(2, '=');
            let key = parts.next().unwrap().trim().to_string();
            let value = parts.next().unwrap_or("").trim().to_string();
            props.insert(key, value);
        }

        Ok(Properties { props })
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.props.get(key)
    }
}

