use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
struct Tag {
    name: String,
    confidence: f64,
}

#[derive(Deserialize)]
struct Metadata {
    height: u32,
    width: u32,
    format: String,
}

#[derive(Deserialize)]
struct JsonData {
    tags: Vec<Tag>,
    requestId: String,
    metadata: Metadata,
    modelVersion: String,
}

fn filtrar_tags(json_data: &str) -> bool {
    // Parsear el JSON
    let data: JsonData = serde_json::from_str(json_data).expect("Error al parsear el JSON");
    
    // Lista de tags relevantes
    let tags_relevantes = vec!["accident", "fire", "crash"];
    
    // Iterar sobre los tags en el JSON
    for tag in data.tags {
        if tags_relevantes.contains(&tag.name.as_str()) && tag.confidence > 0.85 {
            return true;
        }
    }
    
    false
}

fn main() {
    // Leer el archivo JSON
    let json_data = fs::read_to_string("aux_json_accidente.json").expect("No se pudo leer el archivo JSON");

    // Pasar una referencia a json_data
    println!("{}", filtrar_tags(&json_data));  // Debería imprimir true o false dependiendo del contenido del JSON
}