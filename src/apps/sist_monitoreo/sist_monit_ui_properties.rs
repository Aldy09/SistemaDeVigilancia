use std::io::{Error, ErrorKind};

use crate::apps::properties::Properties;

#[derive(Debug)]
pub struct SistMonitUIProperties {
    pub ui_name: String,
    pub ui_cam_img_file: String,
    pub ui_dron_img_file: String,
}

impl SistMonitUIProperties {
    pub fn new(global_properties: Properties) -> Result<Self, Error> {
        let ui_name: String;
        if let Some(ui_name_prop) = global_properties.get("ui_name") {
            ui_name = ui_name_prop.to_owned();
        } else {
            println!("No se encontró la propiedad 'ui_name");
            return Err(Error::new(
                ErrorKind::Other,
                "Falta propiedad sist cams ui.",
            ));
        }

        let ui_cam_img_file: String;
        if let Some(ui_cam_img_file_prop) = global_properties.get("ui_name") {
            ui_cam_img_file = ui_cam_img_file_prop.to_owned();
        } else {
            println!("No se encontró la propiedad 'ui_cam_img_file");
            return Err(Error::new(
                ErrorKind::Other,
                "Falta propiedad sist cams ui.",
            ));
        }

        let ui_dron_img_file: String;
        if let Some(ui_dron_img_file_prop) = global_properties.get("ui_name") {
            ui_dron_img_file = ui_dron_img_file_prop.to_owned();
        } else {
            println!("No se encontró la propiedad 'ui_dron_img_file");
            return Err(Error::new(
                ErrorKind::Other,
                "Falta propiedad sist cams ui.",
            ));
        }

        Ok(Self {
            ui_name,
            ui_cam_img_file,
            ui_dron_img_file,
        })
    }
}
