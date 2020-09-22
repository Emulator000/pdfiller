pub mod document;

pub trait Model: Send + Sync + Unpin {
    fn name() -> &'static str;

    fn id(&self) -> i64;
}
