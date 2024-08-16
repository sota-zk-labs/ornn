pub mod memory_registry;

pub trait Registry {
    fn store(&mut self, key: String, value: String);
    fn load(&self, key: &String) -> Option<&String>;
}
