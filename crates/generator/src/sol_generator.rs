use std::collections::HashMap;
use ast::{ASTNode, Variable};
use registry::Registry;
use crate::Generator;

pub struct SolGenerator {}

impl SolGenerator {
    fn generate_code_recursive(ast: &ASTNode, registry: &Box<dyn Registry>, sub_expression_map: &mut HashMap<String, String>, code: &mut String, temp_var_counter: &mut usize) -> String {
        match registry.load(&ast.to_string()) {
            None => {}
            Some(str) => {
                let node = format!("/*{}*/ mload({})", &ast.to_string(), str);
                return node;
            }
        }

        match ast {
            ASTNode::Variable(var) => {
                let str = match var {
                    Variable::String(str) => {
                        format!("/*{}*/ mload({})", var, match registry.load(str) {
                            None => { "### unknow ###" }
                            Some(str) => { str }
                        })
                    }
                    Variable::U256(u256) => { u256.to_string() }
                };
                str
            }
            ASTNode::Operation(left, op, right) => {
                let left_expr = Self::generate_code_recursive(left, registry, sub_expression_map, code, temp_var_counter);
                let right_expr = Self::generate_code_recursive(right, registry, sub_expression_map, code, temp_var_counter);

                let operation_key = format!("{}{}{}", left_expr, op, right_expr);

                if let Some(result) = sub_expression_map.get(&operation_key) {
                    result.clone()
                } else {
                    let operation_code = match op.as_str() {
                        "+" => format!("addmod({}, {}, PRIME)", left_expr, right_expr),
                        "-" => format!("addmod({}, sub(PRIME, {}), PRIME)", left_expr, right_expr),
                        "*" => format!("mulmod({}, {}, PRIME)", left_expr, right_expr),
                        "^" => format!("expmod({}, {}, PRIME)", left_expr, right_expr),
                        "/" => format!("div({}, {})", left_expr, right_expr),
                        "**" => format!("### pow pow con cac ###({}, {})", left_expr, right_expr),

                        _ => panic!("Unsupported operator"),
                    };

                    *temp_var_counter += 1;
                    let temp_var = format!("val_{}", temp_var_counter);

                    code.push_str(&format!("let {} := {}\n", temp_var, operation_code));
                    temp_var
                }
            }
        }
    }
}


impl Generator<ASTNode> for SolGenerator {
    fn generate(ast: &ASTNode, registry: &Box<dyn Registry>) -> (String, usize) {
        let mut sub_expression_map = HashMap::new();
        let mut code = String::new();
        let mut temp_var_counter = 0;
        let str = Self::generate_code_recursive(ast, registry, &mut sub_expression_map, &mut code, &mut temp_var_counter);
        if temp_var_counter == 0 {
            return (str, 0)
            // return format!("{{\nlet val := {}\nmstore({}, val)\n}}", str, "0x00")
        }
        (code, temp_var_counter)
        // format!("{{\n{}\nlet val := val_{}\nmstore({})\n}}", code, temp_var_counter, "0x00")
        // code
    }
}

#[cfg(test)]
mod tests {
    use parser::ast_parser::ASTParser;
    use parser::Parser;
    use registry::memory_registry::MemoryRegistry;
    use super::*;

    #[test]
    fn single_var() {
        let expression = "domains[0]";
        let ast = ASTParser::parse(expression);

        let mut registry: Box<dyn Registry> = Box::new(MemoryRegistry {
            memory: HashMap::new()
        });

        registry.store("domains[0]".to_string(), "0x20".to_string());

        let (solidity_code, _) = SolGenerator::generate(&ast, &registry);
        println!("ast: {}", ast);
        println!("{}", solidity_code);
    }

    #[test]
    fn simple_expression() {
        let expression = "column0_row0 - (column0_row1 + column0_row1)";
        let ast = ASTParser::parse(expression);

        let mut registry: Box<dyn Registry> = Box::new(MemoryRegistry {
            memory: HashMap::new()
        });
        registry.store("column0_row0".to_string(), "0x00".to_string());
        registry.store("column0_row1".to_string(), "0x00".to_string());

        let (solidity_code, _) = SolGenerator::generate(&ast, &registry);
        println!("ast: {}", ast);
        println!("{}", solidity_code);
    }

    #[test]
    fn simple_registry() {
        let expression = "point^(trace_length / 128) - trace_generator^(3 * trace_length / 4)";
        let ast = ASTParser::parse(expression);

        let mut registry: Box<dyn Registry> = Box::new(MemoryRegistry {
            memory: HashMap::new()
        });
        registry.store("point".to_string(), "0x00".to_string());
        registry.store("trace_length".to_string(), "0x00".to_string());
        registry.store("trace_generator".to_string(), "0x00".to_string());

        registry.store("(point^(trace_length/128))".to_string(), "0x20".to_string());
        registry.store("(trace_generator^((3*trace_length)/4))".to_string(), "0x20".to_string());

        let (solidity_code, _) = SolGenerator::generate(&ast, &registry);
        println!("ast: {}", ast);
        println!("{}", solidity_code);
    }

    #[test]
    fn large_expression() {
        let expression = "column1_row0 + column1_row2 * 2 + column1_row4 * 4 + column1_row6 * 8 + column1_row8 * 18446744073709551616 + column1_row10 * 36893488147419103232 + column1_row12 * 73786976294838206464 + column1_row14 * 147573952589676412928";
        let ast = ASTParser::parse(expression);

        let mut registry: Box<dyn Registry> = Box::new(MemoryRegistry {
            memory: HashMap::new()
        });

        registry.store("column1_row0".to_string(), "0x00".to_string());
        registry.store("column1_row2".to_string(), "0x00".to_string());
        registry.store("column1_row4".to_string(), "0x00".to_string());
        registry.store("column1_row6".to_string(), "0x00".to_string());
        registry.store("column1_row8".to_string(), "0x00".to_string());
        registry.store("column1_row10".to_string(), "0x00".to_string());
        registry.store("column1_row12".to_string(), "0x00".to_string());
        registry.store("column1_row14".to_string(), "0x00".to_string());

        let (solidity_code, _) = SolGenerator::generate(&ast, &registry);
        println!("ast: {}", ast);
        println!("{}", solidity_code);
    }

    // let expression = "(point^(trace_length / 128) - trace_generator^(trace_length / 64)) * (point^(trace_length / 128) - trace_generator^(trace_length / 32)) * (point^(trace_length / 128) - trace_generator^(3 * trace_length / 64)) * (point^(trace_length / 128) - trace_generator^(trace_length / 16)) * (point^(trace_length / 128) - trace_generator^(5 * trace_length / 64)) * (point^(trace_length / 128) - trace_generator^(3 * trace_length / 32)) * (point^(trace_length / 128) - trace_generator^(7 * trace_length / 64)) * (point^(trace_length / 128) - trace_generator^(trace_length / 8)) * (point^(trace_length / 128) - trace_generator^(9 * trace_length / 64)) * (point^(trace_length / 128) - trace_generator^(5 * trace_length / 32)) * (point^(trace_length / 128) - trace_generator^(11 * trace_length / 64)) * (point^(trace_length / 128) - trace_generator^(3 * trace_length / 16)) * (point^(trace_length / 128) - trace_generator^(13 * trace_length / 64)) * (point^(trace_length / 128) - trace_generator^(7 * trace_length / 32)) * (point^(trace_length / 128) - trace_generator^(15 * trace_length / 64)) * domain8";
    // let expression = "point^(trace_length / 128) - trace_generator^(3 * trace_length / 4)";
    // let expression = " 1 - (cpu__decode__opcode_range_check__bit_2 + cpu__decode__opcode_range_check__bit_4 + cpu__decode__opcode_range_check__bit_3)";
    // let expression = "domains[0]";
}



