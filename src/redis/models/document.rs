use crate::redis::models::Model;

#[derive(Serialize)]
pub struct Document {
    pub id: i64,
    pub token: String,
    pub file: String,
}

impl Model for Document {
    fn name() -> &'static str {
        "document"
    }

    fn id(&self) -> i64 {
        self.id
    }
}
