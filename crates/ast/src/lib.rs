use std::fmt::Display;
use primitive_types::U256;

#[derive(Debug, Clone)]
pub enum Variable {
    String(String),
    U256(U256),
}

#[derive(Debug, Clone)]
pub enum ASTNode {
    Variable(Variable),
    Operation(Box<ASTNode>, String, Box<ASTNode>),
}

impl From<String> for Variable {
    fn from(value: String) -> Self {
        match U256::from_str_radix(&value, 10) {
            Ok(u256) => { Variable::U256(u256) }
            Err(_) => { Variable::String(value) }
        }
    }
}

impl Display for Variable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Variable::String(str) => { str }
            Variable::U256(u256) => &{ u256.to_string() }
        };
        write!(f, "{}", str)
    }
}

impl Display for ASTNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            ASTNode::Variable(var) => var.to_string(),
            ASTNode::Operation(left, op, right) => {
                let op = match op.as_str() {
                    "+" | "-" | "*" | "/" | "^" | "**" => {
                        op
                    }
                    _ => {
                        panic!("Invalid operator")
                    }
                };
                format!("({}{}{})", left.to_string(), op, right.to_string())
            }
        };
        write!(f, "{}", str)
    }
}




