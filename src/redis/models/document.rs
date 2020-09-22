use crate::redis::models::Model;

#[derive(Clone, Serialize, Deserialize)]
pub struct Document {
    pub token: String,
    pub file: String,
}

impl Model for Document {
    fn name() -> &'static str {
        "document"
    }

    fn key(&self) -> String {
        format!(
            "{}_{}",
            Self::model_key::<Self, _>(Some(&self.token)),
            &self.file
        )
    }
}
