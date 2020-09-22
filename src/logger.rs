pub struct Logger {}

impl Logger {
    pub fn log<S: AsRef<str>>(text: S) {
        println!("{}", text.as_ref());
    }
}
