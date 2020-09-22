use crate::redis::wrapper::RedisWrapper;

#[derive(Clone)]
pub struct Data {
    pub redis: RedisWrapper,
}

impl Data {
    pub fn new(redis: RedisWrapper) -> Self {
        Data { redis }
    }
}
