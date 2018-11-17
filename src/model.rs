use sheet::Sheet;

pub struct Model {
    documents: Vec<Sheet>,
}

impl Model {
    pub fn new() -> Model {
        Model { documents: vec![] }
    }
}
