use ast::ASTNode;

pub trait GeneratorInput {}

impl GeneratorInput for ASTNode {}

pub trait Generator<T>
where
    T: GeneratorInput,
{
    fn generate(ast: &T) -> String;
}

pub struct ASTGenerator {}

impl ASTGenerator {
    fn generate_code_recursive(ast: &ASTNode, stack: &mut Vec<String>, code: &mut String) {
        match ast {
            ASTNode::Variable(var) => {
                let load_code = format!("/*{}*/ mload({})", var, Self::var_to_memory_location(var));
                let val_assignment = format!("val := {}", load_code);
                // code.push_str(&format!("{}\n", val_assignment));
                code.push_str(&format!("{}\n", val_assignment));
                stack.push(load_code);
            }
            ASTNode::Operation(left, op, right) => {
                Self::generate_code_recursive(right, stack, code);
                Self::generate_code_recursive(left, stack, code);

                let left_expr = stack.pop().expect("Stack underflow");
                let right_expr = stack.pop().expect("Stack underflow");

                let operation_code = match op {
                    '+' => format!("addmod({}, {}, PRIME)", left_expr, right_expr),
                    '-' => format!("addmod({}, sub(PRIME, {}), PRIME)", left_expr, right_expr),
                    '*' => format!("mulmod({}, {}, PRIME)", left_expr, right_expr),
                    _ => panic!("Unsupported operator"),
                };

                let val_assignment = format!("val := {}", operation_code);
                // code.push_str(&format!("{}\n", val_assignment));
                code.push_str(&format!("{}\n", val_assignment));
                stack.push("val".to_string());
            }
        }
    }

    fn var_to_memory_location(var: &str) -> String {
        match var {
            "column0_row0" => "0x540".to_string(),
            "column0_row1" => "0x560".to_string(),
            _ => "0x0".to_string(),
            // Add other variable memory locations as needed
            _ => panic!("Unknown variable"),
        }
    }
}

impl Generator<ASTNode> for ASTGenerator {
    fn generate(ast: &ASTNode) -> String {
        let mut stack = vec![];
        let mut code = String::new();
        Self::generate_code_recursive(ast, &mut stack, &mut code);
        format!("{{\nlet val\n{}\nmstore()\n}}", code)
    }
}


// #[cfg(test)]
// mod test {
//     use parser::{ASTParser, Parser};
//     use crates::{ASTGenerator, Generator};
//
//     #[test]
//     fn test() {
//         let expression = "column0_row0 - (column0_row1 + column0_row1)";
//         let ast = ASTParser::parse(expression);
//         println!("AST: {:?}", ast);
//         let rebuilt_expression = ASTGenerator::generate(&ast);
//         println!("Rebuilt Expression: {}", rebuilt_expression);
//     }
// }

