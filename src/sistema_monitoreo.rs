use reqwest;
use std::fs::File;
use std::io::copy;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Define las coordenadas del centro del mapa
    let lat = -25.363;
    let lng = 131.044;

    // Define el nivel de zoom
    let zoom = 10;

    // Define el tamaño del mapa
    let width = 800;
    let height = 600;

    // Define tu clave de API de Mapbox
    let api_key = "tu_clave_de_api";

    // Genera la URL de la API de mapas estáticos de Mapbox
    let url = format!("https://api.mapbox.com/styles/v1/mapbox/streets-v11/static/{},{},{},{}x{}?access_token={}", lng, lat, zoom, width, height, api_key);

    // Descarga la imagen del mapa
    let response = reqwest::get(&url).await?;
    let mut out = File::create("map.png")?;
    copy(&mut response.bytes().await?.as_ref(), &mut out)?;

    Ok(())
}
