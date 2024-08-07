use std::fmt::Display;

#[derive(Debug, Clone)]
pub enum ASTNode {
    Variable(String),
    Operation(Box<ASTNode>, char, Box<ASTNode>),
}

impl Display for ASTNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            ASTNode::Variable(var) => var.clone(),
            ASTNode::Operation(left, op, right) => {
                let op = match op {
                    '^' => "expmod",
                    '+' | '-' => "addmod",
                    '*' | '/' => "mulmod",
                    _ => {
                        panic!("Invalid operator")
                    }
                };
                format!("({}({}, {}))", op, left.to_string(), right.to_string())
            }
        };
        write!(f, "{}", str)
    }
}




