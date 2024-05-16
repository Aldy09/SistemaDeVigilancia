struct Camera {
    guid: String,
    coord_x: i32,
    coord_y: i32,
}

impl Camera {
    fn new(guid: String, coord_x: i32, coord_y: i32) -> Self {
        Self {
            guid,
            coord_x,
            coord_y,
        }
    }

    fn display(&self) {
        println!("GUID: {}", self.guid);
        println!("Coordenada X: {}", self.coord_x);
        println!("Coordenada Y: {}\n", self.coord_y);
    }
}
