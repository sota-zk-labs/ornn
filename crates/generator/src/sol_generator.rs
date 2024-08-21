use std::collections::HashMap;
use primitive_types::U256;
use tera::Context;
use parser::ast_parser::ASTParser;
use parser::Parser;
use structure::ast::{ASTNode, Variable};
use registry::Registry;
use structure::comment::Comment;
use crate::{Generator, GeneratorInput, GeneratorOutput};

pub struct SolGenerator {
    pub data: ContractData,
}

pub struct ContractData {
    pub memory_layout: Vec<HashMap<String, String>>,
    pub instructions: String,
    pub expmods: String,
    pub domains: String,
    pub denominators: String,
    pub denominator_invs: String,
    pub compositions: String,
    pub registry: Box<dyn Registry>,
    pub domain_registry: Box<dyn Registry>,
    pub denom_registry: Box<dyn Registry>,
    pub nume_registry: Box<dyn Registry>,
    pub ctx: Context,
}

impl SolGenerator {
    fn generate_code_recursive(&self, ast: &ASTNode, code: &mut String, temp_var_counter: &mut usize) -> String {
        let registry = &self.data.registry;
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
                let left_expr = self.generate_code_recursive(left, code, temp_var_counter);
                let right_expr = self.generate_code_recursive(right, code, temp_var_counter);

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

impl Generator<ASTNode, (String, usize)> for SolGenerator {
    fn generate(&mut self, ast: &ASTNode) -> (String, usize) {
        let mut code = String::new();
        let mut temp_var_counter = 0;
        let str = self.generate_code_recursive(ast, &mut code, &mut temp_var_counter);
        let result = match temp_var_counter {
            0 => str,
            _ => code
        };
        (result, temp_var_counter)
    }
}

impl GeneratorOutput for (String, usize) {}
impl GeneratorOutput for () {}
impl GeneratorInput for Comment {}

impl Generator<Comment, ()> for SolGenerator {
    fn generate(&mut self, comment: &Comment) -> () {
        match comment {
            Comment::Memory(slot, start, end) => {
                let mut map: HashMap<String, String> = HashMap::new();
                map.insert("start".to_string(), format!("{:#x}", start));
                map.insert("end".to_string(), format!("{:#x}", end));
                map.insert("description".to_string(), slot.to_string());

                self.data.memory_layout.push(map);

                // intermediate_value
                if end - start == U256::from(32) {
                    let mut key = slot.trim_start_matches("intermediate_value/");
                    key = key.trim_start_matches("periodic_column/");

                    self.data.registry.store(format!("{}", slot.trim_start_matches("intermediate_value/")), format!("{:#x}", start));
                    let key = key.replace("/", "__");
                    self.data.registry.store(key, format!("{:#x}", start));
                    ()
                }
                //
                let mut i = start.clone();
                let mut counter = 0;
                while i < *end {
                    self.data.registry.store(format!("{}{}", slot.trim_end_matches('s'), counter), format!("{:#x}", start + U256::from(counter) * U256::from(32)));
                    self.data.registry.store(format!("{}[{}]", slot, counter), format!("{:#x}", start + U256::from(counter) * U256::from(32)));
                    i += U256::from(32);
                    counter += 1;
                }
                self.data.registry.store(slot.clone(), format!("{:#x}", start));
            }
            Comment::Instruction(left, right, op) => {
                // process the expression
                let ast = ASTParser::parse(right.as_str());
                let (code, count) = SolGenerator::generate(self, &ast);

                // calculate the address
                let (name, index) = Comment::extract_left(&left);
                let block_name = name.clone();
                let name = match index {
                    0 => name.to_string(),
                    _ => format!("{}{}", name.trim_end_matches('s'), index)
                };
                let slot = match self.data.registry.load(&name) {
                    None => {
                        println!("### warn ### No memory slot found for: {}", name);
                        return;
                    }
                    Some(address) => address
                }.to_string();

                self.data.registry.store(ast.to_string(), slot.clone());

                let a = match count {
                    0 => format!("{{\nlet val := {}\nmstore({}, val)\n}}", code, slot),
                    _ => format!("{{\n{}\nmstore({}, val_{})\n}}", code, slot, count)
                };
                // code

                let a = format!("// {} = {}\n{}\n", left, right, a);

                match block_name {
                    "expmods" => self.data.expmods.push_str(a.as_str()),
                    "domains" => self.data.domains.push_str(a.as_str()),
                    "denominators" => self.data.denominators.push_str(a.as_str()),
                    "compositions" => self.data.compositions.push_str(a.as_str()),
                    _ => self.data.instructions.push_str(a.as_str())
                }
            }
            Comment::None => {}
            Comment::ConstraintData(index, type_, constraints) => {
                let (registry, name) = match type_.as_str() {
                    "Numerator" => (&mut self.data.nume_registry, "domains"),
                    "Denominator" => (&mut self.data.denom_registry, "denominator_invs"),
                    _ => (&mut self.data.denom_registry, "")
                };

                for constraint in constraints {
                    registry.store(constraint.to_string(), format!("{}[{}]", name, index));
                }
            }
            Comment::UpdateConstrainsData(denom, domain) => {
                self.data.domain_registry.store(format!("denominator_invs[{}]", domain.to_string()), format!("denominator_invs[{}]", denom.to_string()));
            }
            Comment::Constraint(name, expr) => {
                // process the expression
                let ast = ASTParser::parse(expr.as_str());
                let (code, count) = SolGenerator::generate(self, &ast);

                // &data.registry.store(e.left.clone(), slot.to_string());

                let numerator = match self.data.nume_registry.load(name) {
                    None => {
                        println!("### warn ### No numerator found for: {}", name);
                        "1"
                    }
                    Some(str) => str
                };

                let numerator = match numerator {
                    "1" =>
                        "".to_string(),
                    _ => {
                        let slot = match self.data.registry.load(&numerator.to_string()) {
                            None => {
                                println!("### warn ### No memory slot found for: {}", numerator);
                                return;
                            }
                            Some(address) => address
                        };
                        format!("// Numerator\n// val *= numerator\nval := mulmod(val, mload({}), PRIME)\n", slot)
                    }
                };

                let denominator = match self.data.denom_registry.load(name) {
                    None => {
                        println!("### warn ### No denominator found for: {}", name);
                        "1"
                    }
                    Some(str) => str
                };

                let denominator = match denominator {
                    "1" =>
                        "".to_string(),
                    _ => {
                        let invs = match self.data.domain_registry.load(&denominator.to_string()) {
                            None => denominator.to_string(),
                            Some(str) => str.to_string()
                        };

                        let slot = match self.data.registry.load(&invs) {
                            None => {
                                println!("### warn ### No memory slot found for: {}", denominator);
                                return;
                            }
                            Some(address) => address
                        };
                        format!("// Denominator\n// val *= denominator inverse\n val := mulmod(val, mload({}), PRIME)\n", slot)
                    }
                };

                let mut a = match count {
                    0 => format!("{{\nlet val :={}\n{}\n{}\n", code, numerator, denominator),
                    _ => format!("{{\n{}\nlet val := val_{}\n{}\n{}\n", code, count, numerator, denominator)
                };

                a.push_str("res := addmod(res, mulmod(val, composition_alpha_pow, PRIME), PRIME)\ncomposition_alpha_pow := mulmod(composition_alpha_pow, composition_alpha, PRIME)\n");
                // code

                let a = format!("\n//Constraint expression for {}: {}\n{}\n}}\n", name, expr, a);

                self.data.compositions.push_str(a.as_str());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use parser::ast_parser::ASTParser;
    use parser::Parser;
    use registry::hashmap_registry::HashMapRegistry;
    use super::*;

    #[test]
    fn single_var() {
        let expression = "domains[0]";
        let ast = ASTParser::parse(expression);

        let mut registry: Box<dyn Registry> = Box::new(HashMapRegistry {
            memory: HashMap::new()
        });

        registry.store("domains[0]".to_string(), "0x20".to_string());

        let mut generator = SolGenerator {
            data: ContractData {
                memory_layout: Vec::new(),
                instructions: String::new(),
                expmods: String::new(),
                domains: String::new(),
                denominators: String::new(),
                denominator_invs: String::new(),
                compositions: String::new(),
                registry,
                domain_registry: Box::new(HashMapRegistry {
                    memory: HashMap::new()
                }),
                denom_registry: Box::new(HashMapRegistry {
                    memory: HashMap::new()
                }),
                nume_registry: Box::new(HashMapRegistry {
                    memory: HashMap::new()
                }),
                ctx: Context::new(),
            }
        };

        let (solidity_code, _) = generator.generate(&ast);
        println!("structure: {}", ast);
        println!("{}", solidity_code);
    }

    #[test]
    fn simple_expression() {
        let expression = "column0_row0 - (column0_row1 + column0_row1)";
        let ast = ASTParser::parse(expression);

        let mut registry: Box<dyn Registry> = Box::new(HashMapRegistry {
            memory: HashMap::new()
        });
        registry.store("column0_row0".to_string(), "0x00".to_string());
        registry.store("column0_row1".to_string(), "0x00".to_string());

        let mut generator = SolGenerator {
            data: ContractData {
                memory_layout: Vec::new(),
                instructions: String::new(),
                expmods: String::new(),
                domains: String::new(),
                denominators: String::new(),
                denominator_invs: String::new(),
                compositions: String::new(),
                registry,
                domain_registry: Box::new(HashMapRegistry {
                    memory: HashMap::new()
                }),
                denom_registry: Box::new(HashMapRegistry {
                    memory: HashMap::new()
                }),
                nume_registry: Box::new(HashMapRegistry {
                    memory: HashMap::new()
                }),
                ctx: Context::new(),
            }
        };

        let (solidity_code, _) = generator.generate(&ast);
        println!("structure: {}", ast);
        println!("{}", solidity_code);
    }

    #[test]
    fn simple_registry() {
        let expression = "point^(trace_length / 128) - trace_generator^(3 * trace_length / 4)";
        let ast = ASTParser::parse(expression);

        let mut registry: Box<dyn Registry> = Box::new(HashMapRegistry {
            memory: HashMap::new()
        });
        registry.store("point".to_string(), "0x00".to_string());
        registry.store("trace_length".to_string(), "0x00".to_string());
        registry.store("trace_generator".to_string(), "0x00".to_string());

        registry.store("(point^(trace_length/128))".to_string(), "0x20".to_string());
        registry.store("(trace_generator^((3*trace_length)/4))".to_string(), "0x20".to_string());

        let mut generator = SolGenerator {
            data: ContractData {
                memory_layout: Vec::new(),
                instructions: String::new(),
                expmods: String::new(),
                domains: String::new(),
                denominators: String::new(),
                denominator_invs: String::new(),
                compositions: String::new(),
                registry,
                domain_registry: Box::new(HashMapRegistry {
                    memory: HashMap::new()
                }),
                denom_registry: Box::new(HashMapRegistry {
                    memory: HashMap::new()
                }),
                nume_registry: Box::new(HashMapRegistry {
                    memory: HashMap::new()
                }),
                ctx: Context::new(),
            }
        };

        let (solidity_code, _) = generator.generate(&ast);
        println!("structure: {}", ast);
        println!("{}", solidity_code);
    }

    #[test]
    fn large_expression() {
        let expression = "column1_row0 + column1_row2 * 2 + column1_row4 * 4 + column1_row6 * 8 + column1_row8 * 18446744073709551616 + column1_row10 * 36893488147419103232 + column1_row12 * 73786976294838206464 + column1_row14 * 147573952589676412928";
        let ast = ASTParser::parse(expression);

        let mut registry: Box<dyn Registry> = Box::new(HashMapRegistry {
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

        let mut generator = SolGenerator {
            data: ContractData {
                memory_layout: Vec::new(),
                instructions: String::new(),
                expmods: String::new(),
                domains: String::new(),
                denominators: String::new(),
                denominator_invs: String::new(),
                compositions: String::new(),
                registry,
                domain_registry: Box::new(HashMapRegistry {
                    memory: HashMap::new()
                }),
                denom_registry: Box::new(HashMapRegistry {
                    memory: HashMap::new()
                }),
                nume_registry: Box::new(HashMapRegistry {
                    memory: HashMap::new()
                }),
                ctx: Context::new(),
            }
        };

        let (solidity_code, _) = generator.generate(&ast);
        println!("structure: {}", ast);
        println!("{}", solidity_code);
    }

    // let expression = "(point^(trace_length / 128) - trace_generator^(trace_length / 64)) * (point^(trace_length / 128) - trace_generator^(trace_length / 32)) * (point^(trace_length / 128) - trace_generator^(3 * trace_length / 64)) * (point^(trace_length / 128) - trace_generator^(trace_length / 16)) * (point^(trace_length / 128) - trace_generator^(5 * trace_length / 64)) * (point^(trace_length / 128) - trace_generator^(3 * trace_length / 32)) * (point^(trace_length / 128) - trace_generator^(7 * trace_length / 64)) * (point^(trace_length / 128) - trace_generator^(trace_length / 8)) * (point^(trace_length / 128) - trace_generator^(9 * trace_length / 64)) * (point^(trace_length / 128) - trace_generator^(5 * trace_length / 32)) * (point^(trace_length / 128) - trace_generator^(11 * trace_length / 64)) * (point^(trace_length / 128) - trace_generator^(3 * trace_length / 16)) * (point^(trace_length / 128) - trace_generator^(13 * trace_length / 64)) * (point^(trace_length / 128) - trace_generator^(7 * trace_length / 32)) * (point^(trace_length / 128) - trace_generator^(15 * trace_length / 64)) * domain8";
    // let expression = "point^(trace_length / 128) - trace_generator^(3 * trace_length / 4)";
    // let expression = " 1 - (cpu__decode__opcode_range_check__bit_2 + cpu__decode__opcode_range_check__bit_4 + cpu__decode__opcode_range_check__bit_3)";
    // let expression = "domains[0]";
}



