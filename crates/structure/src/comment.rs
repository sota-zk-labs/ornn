use std::collections::HashMap;
use primitive_types::U256;
use regex::Regex;

#[derive(Clone, Debug)]
pub enum Comment {
    Memory(String, U256, U256), // (slot, start, end)
    Instruction(String, String, String), // (left, right, op)
    ConstraintData(usize, String, Vec<String>), // (index, type, name)
    UpdateConstrainsData(usize, usize), // (denom, domain)
    Constraint(String, String), // (name, expression)
    None,
}

impl Comment {
    pub fn extract_left(left: &str) -> (&str, usize) {
        let re = Regex::new(r"(?P<name>[a-zA-Z0-9_/]+)\[(?P<index>\d+)\]").unwrap();
        if let Some(captures) = re.captures(left) {
            let name = captures.name("name").unwrap().as_str();
            let index = captures.name("index").unwrap().as_str();
            (name, index.parse().unwrap())
        } else {
            (left.clone(), 0)
        }
    }
}