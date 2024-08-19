use std::collections::HashMap;
use crate::Registry;

pub struct HashMapRegistry {
    pub memory: HashMap<String, String>,
}

impl Registry for HashMapRegistry {
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
