use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, Error, ErrorKind};
use std::path::Path;

pub struct Properties {
    props: HashMap<String, String>,
}

impl Properties {
    pub fn new(file_path: &str) -> Result<Self, Error> {
        let path = Path::new(file_path);
        let file = File::open(path)?;
        let reader = io::BufReader::new(file);

        let mut props = HashMap::new();

        for line in reader.lines() {
            let line = line?;
            let mut parts = line.splitn(2, '=');

            let opt_key = parts.next();
            let opt_value = parts.next();

            // Verificamos el caso de error primero
            if opt_key.is_none() || opt_value.is_none() {
                return Err(Error::new(ErrorKind::InvalidInput, ""));
            }

            // En este punto ya sabemos que no fue error
            if let Some(key) = opt_key {
                let key_to_store = key.trim().to_string();
                if let Some(value) = opt_value {
                    let value_to_store = value.trim().to_string();

                    // Se guarda el par clave - valor
                    props.insert(key_to_store, value_to_store);
                }
            }
        }

        Ok(Properties { props })
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.props.get(key)
    }
}
