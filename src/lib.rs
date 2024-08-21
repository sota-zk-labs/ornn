use generator::Generator;
use lazy_static::lazy_static;
use parser::Parser;
use regex::Regex;
use registry::Registry;
use serde_json::{to_value, Value};
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};
use structure::ast::ASTNode;
use tera::{try_get_value, Context, Tera};

pub trait Generate {
    fn generate(&self) -> String;
}

pub struct Template {
    name: String,
    ctx: Box<Context>,
}
// pub enum Node {
//     Block(Vec<Box<Node>>),
//     String(String),
//     Template(Template),
//     Expression(Instruction),
// }
//
// impl Generate for Node {
//     fn generate(&self) -> String {
//         match self {
//             Node::String(s) => s.to_string(),
//             Node::Template(_) => "template".to_string(),
//             Node::Expression(e) => e.generate(),
//             Node::Block(nodes) => {
//                 let mut result = String::new();
//                 for node in nodes {
//                     result.push_str(node.generate().as_str());
//                 }
//                 result
//             }
//         }
//     }
// }
pub struct Block;

impl Generate for Instruction {
    fn generate(&self) -> String {
        format!("{} {} {}", self.left, self.operator, self.right)
    }
}

impl Template {
    fn generate(&self) -> String {
        match TEMPLATES.render("name", &self.ctx) {
            Ok(s) => {
                s
            }
            Err(e) => {
                println!("### warn ###: {}", e);
                let mut cause = e.source();
                while let Some(e) = cause {
                    println!("Reason: {}", e);
                    cause = e.source();
                }
                format!("### warn ### render template error {}", e)
            }
        }
    }
}

#[derive(Clone, Debug)]
struct Instruction {
    left: String,
    right: String,
    operator: String,
}

pub struct Expression(ASTNode);

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let mut tera = match Tera::new("examples/cpu-constraint-poly/templates/**/*") {
            Ok(t) => t,
            Err(e) => {
                println!("Parsing error(s): {}", e);
                ::std::process::exit(1);
            }
        };
        tera.autoescape_on(vec![]);
        tera.register_filter("do_nothing", do_nothing_filter);
        tera
    };
}

pub fn do_nothing_filter(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    let s = try_get_value!("do_nothing_filter", "value", String, value);
    Ok(to_value(s).unwrap())
}


fn read(sol_path: &str) -> String {
    let sol_file = File::open(sol_path).expect("Failed to open Solidity file");
    let mut sol_code = String::new();
    std::io::BufReader::new(sol_file).read_to_string(&mut sol_code).expect("Failed to read Solidity file");

    sol_code
}

fn extract_comment(code: &String) -> Vec<String> {
    // Regex for single-line comments
    let single_line_re = Regex::new(r"//.*").unwrap();
    let mut comments = Vec::new();
    // Extract single-line comments
    for mut single_comment in single_line_re.find_iter(&code) {
        // Trim the last "." if it exists
        comments.push(single_comment.as_str().trim_end_matches('.').to_string());
    }
    comments
}

fn extract_oods_values(code: &String) -> HashMap<String, String> {
    let re = Regex::new(r"/\*(?P<comment>[a-zA-Z0-9_]+)\*/\s*mload\((?P<address>0x[0-9a-fA-F]+)\)").unwrap();
    let mut oods = HashMap::new();
    // Extract oods
    for mut captures in re.captures_iter(&code) {
        let comment = captures.name("comment").unwrap().as_str().to_string();
        let address = captures.name("address").unwrap().as_str().to_string();
        oods.insert(comment, address);
    }
    oods
}

fn extract_denom_invs(code: &String) -> HashMap<String, String> {
    let re = Regex::new(r"/\*denominator_invs\[(\d+)\]\*/\s*mload\((0x[a-fA-F0-9]+)\)").unwrap();
    let mut denom_invs = HashMap::new();

    for captures in re.captures_iter(&code) {
        let index = captures.get(1).unwrap().as_str();
        let address = captures.get(2).unwrap().as_str();
        denom_invs.insert(format!("denominator_invs[{}]", index), address.to_string());
    }
    denom_invs
}

#[cfg(test)]
mod test {
    use crate::{extract_comment, extract_denom_invs, extract_oods_values, read, TEMPLATES};
    use primitive_types::U256;
    use regex::Regex;
    use registry::hashmap_registry::HashMapRegistry;
    use registry::Registry;
    use std::collections::HashMap;
    use std::error::Error;
    use std::fs;
    use std::fs::File;
    use std::io::Write;
    use structure::comment::Comment;
    use tera::Context;
    use generator::{ContractData, Generator};
    use generator::move_generator::MoveGenerator;

    #[test]
    fn main() {
        let mut generator = MoveGenerator {
            data: ContractData {
                memory_layout: Vec::new(),
                instructions: String::new(),
                expmods: String::new(),
                domains: String::new(),
                denominators: String::new(),
                denominator_invs: String::new(),
                compositions: String::new(),
                registry: Box::new(HashMapRegistry {
                    memory: HashMap::new()
                }),
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

        &generator.data.registry.store("point".to_string(), "0x440".to_string());

        let code = read("examples/cpu-constraint-poly/CpuConstraintPoly.sol");
        let comments = extract_comment(&code);

        // find the oods value's memory addresses
        let oods = extract_oods_values(&code);
        for (key, value) in oods {
            generator.data.registry.store(key, value);
        }

        // find the denominator_invs's memory addresses
        let oods = extract_denom_invs(&code);
        println!("{:?}", oods);
        for (key, value) in oods {
            generator.data.registry.store(key, value);
        }

        let mut current_processor = "";

        let mut current_domain = 0;

        for comment in comments {
            let mut c = vec![];

            current_processor = match comment.as_str() {
                "// The Memory map during the execution of this contract is as follows:" => "layout",
                "// Prepare expmods for denominators and numerators" => "expmods",
                "// Compute domains" => "domains",
                "// Prepare denominators for batch inverse" => "denominators",
                "// Compute the inverses of the denominators into denominatorInvs using batch inverse" => "denominator_invs",
                "// Compute the result of the composition polynomial" => "compositions",
                _ => {
                    current_processor
                }
            };

            match current_processor {
                "layout" => {
                    let patterns: Vec<(Regex, Box<dyn Fn(&regex::Captures) -> Comment>)> = vec![
                        // [0x0, 0x20) - periodic_column/pedersen/points/x.
                        (Regex::new(r"\[(0x[0-9a-fA-F]+),\s*(0x[0-9a-fA-F]+)\)\s*-\s*(.+)").unwrap(),
                         Box::new(|captures: &regex::Captures| {
                             let start_value = &captures[1];
                             let end_value = &captures[2];
                             let description = &captures[3].trim();

                             Comment::Memory(
                                 description.to_string(),
                                 U256::from_str_radix(start_value, 16).unwrap(),
                                 U256::from_str_radix(end_value, 16).unwrap(),
                             )
                         })),
                    ];
                    for (regex, processor) in patterns {
                        if let Some(captures) = regex.captures(&comment.clone()) {
                            c.push(processor(&captures));
                        }
                    }
                }
                "expmods" => {
                    let patterns: Vec<(Regex, Box<dyn Fn(&regex::Captures) -> Comment>)> = vec![
                        // expmods[0] = point^(trace_length / 2048).
                        (Regex::new(r"//\s*(.+?)\s*([+\-*/]?=)\s*(.+)").unwrap(),
                         Box::new(|captures: &regex::Captures| {
                             let left = &captures[1];

                             let operator = &captures[2];
                             let right = &captures[3];
                             Comment::Instruction(
                                 left.to_string(),
                                 right.to_string(),
                                 operator.to_string(),
                             )
                         })),
                        (Regex::new(r"//\s*(.+?)\s*([+\-*/]?=)\s*(.+)").unwrap(),
                         Box::new(|captures: &regex::Captures| {
                             let left = &captures[1];
                             let operator = &captures[2];
                             let right = &captures[3];
                             Comment::Instruction(
                                 left.to_string(),
                                 right.to_string(),
                                 operator.to_string(),
                             )
                         })),
                    ];
                    for (regex, processor) in patterns {
                        if let Some(captures) = regex.captures(&comment.clone()) {
                            c.push(processor(&captures));
                        }
                    }
                }
                "domains" => {
                    let current_domain_clone = current_domain.clone();

                    let patterns: Vec<(Regex, Box<dyn FnMut(&regex::Captures) -> Comment>)> = vec![
                        // expmods[0] = point^(trace_length / 2048).
                        (Regex::new(r"//\s*(.+?)\s*([+\-*/]?=)\s*(.+)").unwrap(),
                         Box::new(|captures: &regex::Captures| {
                             let left = &captures[1];
                             let operator = &captures[2];
                             let right = &captures[3];

                             let left_re = Regex::new(r"(\w+)\[(\d+)\]").unwrap();

                             if let Some(captures) = left_re.captures(left) {
                                 let name = &captures[1];
                                 let index = &captures[2];
                                 if name == "domains" {
                                     current_domain = index.parse::<usize>().unwrap() + 1;
                                 }
                             }

                             Comment::Instruction(
                                 left.to_string(),
                                 right.to_string(),
                                 operator.to_string(), )
                         })),
                        // Denominator for constraints: 'cpu/decode/opcode_range_check/bit', 'diluted_check/permutation/step0', 'diluted_check/step'.
                        (Regex::new(r"//\s*(Denominator|Numerator)\s*for\s*constraints:\s*((?:'[^']*'(?:,\s*)?)*)\s*").unwrap(),
                         Box::new(|captures: &regex::Captures| {
                             let constraint_type = captures[1].to_string();
                             let constraints = captures[2].to_string();

                             let constraints: Vec<String> = constraints.split(", ")
                                 .map(|s| s.trim_matches('\'').to_string())
                                 .collect();

                             Comment::ConstraintData(
                                 current_domain_clone,
                                 constraint_type,
                                 constraints,
                             )
                         })),
                    ];
                    for (regex, mut processor) in patterns {
                        if let Some(captures) = regex.captures(&comment.clone()) {
                            c.push(processor(&captures));
                        }
                    }
                }
                "denominators" => {
                    let patterns: Vec<(Regex, Box<dyn Fn(&regex::Captures) -> Vec<Comment>>)> = vec![
                        // expmods[0] = point^(trace_length / 2048).
                        (Regex::new(r"//\s*(.+?)\s*([+\-*/]?=)\s*(.+)").unwrap(),
                         Box::new(|captures: &regex::Captures| {
                             let left = &captures[1];
                             let operator = &captures[2];
                             let right = &captures[3];

                             let (_, denominator) = Comment::extract_left(left);
                             let (_, domain ) = Comment::extract_left(right);

                             vec![Comment::UpdateConstrainsData(
                                 denominator, domain
                             ), Comment::Instruction(
                                 left.to_string(),
                                 right.to_string(),
                                 operator.to_string(),
                             )]
                         })),
                    ];
                    for (regex, processor) in patterns {
                        if let Some(captures) = regex.captures(&comment.clone()) {
                            c = processor(&captures);
                        }
                    }
                }
                "denominator_invs" => {
                    generator.data.denominator_invs = comment;
                }
                "compositions" => {
                    let patterns: Vec<(Regex, Box<dyn Fn(&regex::Captures) -> Comment>)> = vec![
                        // poseidon/poseidon/partial_rounds_state1_cubed_19 = column7_row77 * column7_row79.
                        (Regex::new(r"//\s*(.+?)\s*([+\-*/]?=)\s*(.+)").unwrap(),
                         Box::new(|captures: &regex::Captures| {
                             let left = &captures[1];
                             let operator = &captures[2];
                             let right = &captures[3];
                             Comment::Instruction(
                                 left.to_string(),
                                 right.to_string(),
                                 operator.to_string(),
                             )
                         })),
                        // Constraint expression for cpu/decode/opcode_range_check/bit: cpu__decode__opcode_range_check__bit_0 * cpu__decode__opcode_range_check__bit_0 - cpu__decode__opcode_range_check__bit_0.
                        (Regex::new(r"//\s*Constraint\s+expression\s+for\s+([a-zA-Z0-9/_]+)\s*:\s*(.+)").unwrap(),
                         Box::new(|captures: &regex::Captures| {
                             let name = &captures[1];
                             let expression = &captures[2];
                             Comment::Constraint(name.to_string(), expression.to_string())
                         })),
                    ];
                    for (regex, processor) in patterns {
                        if let Some(captures) = regex.captures(&comment.clone()) {
                            c.push(processor(&captures));
                        }
                    }
                }
                _ => {}
            }

            for comment in c {
                generator.generate(&comment);
            }
        }

        generator.data.ctx.insert("memory_layout", &generator.data.memory_layout);
        generator.data.ctx.insert("expmods", &generator.data.expmods);
        generator.data.ctx.insert("domains", &generator.data.domains);
        generator.data.ctx.insert("denominators", &generator.data.denominators);
        generator.data.ctx.insert("instructions", &generator.data.instructions);
        generator.data.ctx.insert("compositions", &generator.data.compositions);

        match TEMPLATES.render("move/cpu.move.template", &generator.data.ctx) {
            Ok(s) => {
                fs::create_dir_all("generated").expect("Failed to create directory");
                let mut file = File::create("generated/test.move").expect("Failed to create Move file");
                file.write_all(s.as_bytes()).expect("Failed to write to Move file");
            }
            Err(e) => {
                println!("Error: {}", e);
                let mut cause = e.source();
                while let Some(e) = cause {
                    println!("Reason: {}", e);
                    cause = e.source();
                }
            }
        };
    }

    // #[test]
    // fn demo() {
    //     let code = Node::Block(
    //         vec![
    //             Box::new(Node::String(String::from("Hello"))),
    //             Box::new(Node::String(String::from("\n"))),
    //             Box::new(Node::Template(Template {
    //                 name: "template".to_string(),
    //                 ctx: Box::new(Context::new()),
    //             })),
    //             Box::new(Node::Block(vec![
    //                 Box::new(Node::String(String::from("\nBlock start"))),
    //                 Box::new(Node::Template(Template {
    //                     name: "template".to_string(),
    //                     ctx: Box::new(Context::new()),
    //                 })),
    //                 Box::new(Node::Expression(Instruction {
    //                     left: "left".to_string(),
    //                     right: "right".to_string(),
    //                     operator: "+".to_string(),
    //                 })),
    //                 Box::new(Node::String(String::from("Block end\n"))),
    //             ])),
    //             Box::new(Node::Expression(Instruction {
    //                 left: "left".to_string(),
    //                 right: "right".to_string(),
    //                 operator: "+".to_string(),
    //             })),
    //         ]
    //     );
    //
    //     let s = format!("{}", code.generate());
    //
    //     fs::create_dir_all("generated").expect("Failed to create directory");
    //     let mut file = File::create("generated/test").expect("Failed to create Move file");
    //     file.write_all(s.as_bytes()).expect("Failed to write to Move file");
    // }

    #[test]
    fn test_regex() {
        let input = "//constraints expression for memory/multi_column_perm/perm/step0: some_expression_here";

        let re = Regex::new(r"\s*(.+?)\s*:\s*(.*)").unwrap();

        if let Some(captures) = re.captures(input) {
            let lhs = captures.get(1).unwrap().as_str();
            let rhs = captures.get(2).unwrap().as_str();
            println!("LHS: '{}', RHS: '{}'", lhs, rhs);
        } else {
            println!("No match found.");
        }
    }
    #[test]
    fn test_regex2() {
        let text = r#"
        // Denominator for constraints: 'cpu/decode/opcode_range_check/bit', 'diluted_check/permutation/step0', 'diluted_check/step'
        // Numerator for constraints: 'cpu/decode/opcode_range_check/bit'
    "#;

        let re = Regex::new(r"//\s*(Denominator|Numerator)\s*for\s*constraints:\s*((?:'[^']*'(?:,\s*)?)*)\s*").unwrap();

        for captures in re.captures_iter(text) {
            let constraint_type = captures.get(1).unwrap().as_str();
            let constraints_str = captures.get(2).unwrap().as_str();

            // Split the constraints by ", " and remove the quotes
            let constraints: Vec<String> = constraints_str.split(", ")
                .map(|s| s.trim_matches('\'').to_string())
                .collect();

            println!("Type: '{}', Constraints: {:?}", constraint_type, constraints);
        }
    }

    #[test]
    fn test_regex3() {
        let inputs = vec![
            "// domains[15] = point^(trace_length / 1024) - 1",
            "// domains[0] += point^trace_length - 1",
            "// denominators[0] *= domains[0]",
        ];

        let re = Regex::new(r"//\s*(.+?)\s*([+\-*/]?=)\s*(.+)").unwrap();

        for input in inputs {
            if let Some(captures) = re.captures(input) {
                let lhs = captures.get(1).unwrap().as_str();
                let operator = captures.get(2).unwrap().as_str();
                let rhs = captures.get(3).unwrap().as_str();
                println!("LHS: '{}', Operator: '{}', RHS: '{}'", lhs, operator, rhs);
            } else {
                println!("No match found for: {}", input);
            }
        }
    }
}