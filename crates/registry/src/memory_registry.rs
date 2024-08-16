use std::collections::HashMap;
use crate::Registry;

pub struct MemoryRegistry {
    pub memory: HashMap<String, String>,
}

impl Registry for MemoryRegistry {
    fn store(&mut self, key: String, value: String) {
        self.memory.insert(key, value);
    }

    fn load(&self, key: &String) -> Option<&String> {
        self.memory.get(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {}
}
