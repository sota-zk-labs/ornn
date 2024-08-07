#[macro_use]
extern crate tera;
#[macro_use]
extern crate lazy_static;
extern crate serde_json;

use std::string::String;
use std::collections::HashMap;

use serde_json::value::{to_value, Value};
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use primitive_types::U256;
use regex::Regex;
use tera::{Context, Result, Tera};

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

pub fn do_nothing_filter(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("do_nothing_filter", "value", String, value);
    Ok(to_value(s).unwrap())
}

pub enum Op {
    Expmod,
    Expmod8,
    ExpmodTraceGen,
    ExpmodTraceGenOdd,
    ExpmodTraceGenSub,
    Domain,
}

struct Expression {
    left: String,
    right: String,
}

struct Instruction {
    op: Op,
    data: Vec<String>,
    raw: Expression,
}

#[derive(Copy, Clone, Debug)]
struct MemoryRange {
    start: U256,
    end: U256,
}

struct SolidityData {
    memory_layout: HashMap<String, MemoryRange>,
    memory: HashMap<String, String>,
    instructions: Vec<Instruction>,
}

enum ParserOutput {
    Memory(String, MemoryRange),
    Instruction(Instruction),
    None,
}


fn reader(sol_path: &str) -> Vec<String> {
    let sol_file = File::open(sol_path).expect("Failed to open Solidity file");
    let mut sol_code = String::new();
    std::io::BufReader::new(sol_file).read_to_string(&mut sol_code).expect("Failed to read Solidity file");
    // Regex for single-line comments
    let single_line_re = Regex::new(r"//.*").unwrap();
    let mut comments = Vec::new();
    // Extract single-line comments
    for mut single_comment in single_line_re.find_iter(&sol_code) {
        // Trim the last "." if it exists
        comments.push(single_comment.as_str().trim_end_matches('.').to_string());
    }
    comments
}

fn extract_raw_expression(comment: String) -> Expression {
    let re = Regex::new(r"//\s*(.+?)\s*=\s*(.+)").unwrap();

    let caps = re.captures(&comment).unwrap();
    let left_side = &caps[1];
    let right_side = &caps[2];

    return Expression {
        left: left_side.to_string(),
        right: right_side.to_string(),
    };
}

fn parser(comment: String) -> ParserOutput {
    let patterns: Vec<(Regex, Box<dyn Fn(&regex::Captures) -> ParserOutput>)> = vec![
        // [0x0, 0x20) - periodic_column/pedersen/points/x.
        (Regex::new(r"\[(0x[0-9a-fA-F]+),\s*(0x[0-9a-fA-F]+)\)\s*-\s*(.+)").unwrap(),
         Box::new(|captures: &regex::Captures| {
             let start_value = &captures[1];
             let end_value = &captures[2];
             let description = &captures[3].trim();
             ParserOutput::Memory(description.to_string(), MemoryRange {
                 start: U256::from_str_radix(start_value, 16).unwrap(),
                 end: U256::from_str_radix(end_value, 16).unwrap(),
             })
             // format!("// MOVE: start: {}, end: {}, description: {}", start_value, end_value, description)
         })),
        // expmods[0] = point^(trace_length / 2048).
        (Regex::new(r"//\s*expmods\[(\d+)\]\s*=\s*point\^\(trace_length\s*/\s*(\d+)\)").unwrap(),
         Box::new(|captures: &regex::Captures| {
             let first_value = &captures[1];
             let second_value = &captures[2];
             let data = vec![first_value.to_string(), second_value.to_string()];
             //     U256::from_str_radix(first_value, 10).unwrap(),
             //     U256::from_str_radix(second_value, 10).unwrap(),
             // ];
             let expression = extract_raw_expression(comment.clone());
             ParserOutput::Instruction(Instruction { op: Op::Expmod, data, raw: expression })
             // format!("// MOVE: expmods index: {}, divisor: {}", first_value, second_value)
         })),
        // expmods[8] = point^trace_length.
        (Regex::new(r"//\s*expmods\[(\d+)\]\s*=\s*point\^trace_length").unwrap(),
         Box::new(|captures: &regex::Captures| {
             let first_value = &captures[1];
             let data = vec![first_value.to_string()];
             //     U256::from_str_radix(first_value, 10).unwrap(),
             // ];
             let expression = extract_raw_expression(comment.clone());

             ParserOutput::Instruction(Instruction { op: Op::Expmod8, data, raw: expression })
         })),
        // expmods[9] = trace_generator^(trace_length / 64).
        (Regex::new(r"//\s*expmods\[(\d+)\]\s*=\s*trace_generator\^\(trace_length\s*/\s*(\d+)\)").unwrap(),
         Box::new(|captures: &regex::Captures| {
             let first_value = &captures[1];
             let second_value = &captures[2];
             let data = vec![first_value.to_string(), second_value.to_string()];
             //     U256::from_str_radix(first_value, 10).unwrap(),
             //     U256::from_str_radix(second_value, 10).unwrap(),
             // ];
             let expression = extract_raw_expression(comment.clone());

             ParserOutput::Instruction(Instruction { op: Op::ExpmodTraceGen, data, raw: expression })
         })),
        // expmods[9] = trace_generator^(3 * trace_length / 64).
        (Regex::new(r"//\s*expmods\[(\d+)]\s*=\s*trace_generator\^\(\s*(\d+)\s*\*\s*trace_length\s*/\s*(\d+)").unwrap(),
         Box::new(|captures: &regex::Captures| {
             let first_value = &captures[1];
             let second_value = &captures[2];
             let third_value = &captures[3];
             let data = vec![first_value.to_string(), second_value.to_string(), third_value.to_string()];
             //     U256::from_str_radix(first_value, 10).unwrap(),
             //     U256::from_str_radix(second_value, 10).unwrap(),
             //     U256::from_str_radix(third_value, 10).unwrap(),
             // ];
             let expression = extract_raw_expression(comment.clone());

             ParserOutput::Instruction(Instruction { op: Op::ExpmodTraceGenOdd, data, raw: expression })
         })),
        // expmods[41] = trace_generator^(trace_length - 16).
        (Regex::new(r"//\s*expmods\[(\d+)\]\s*=\s*trace_generator\^\(trace_length\s*-\s*(\d+)\)").unwrap(),
         Box::new(|captures: &regex::Captures| {
             let first_value = &captures[1];
             let second_value = &captures[2];
             let data = vec![first_value.to_string(), second_value.to_string()];
             //     U256::from_str_radix(first_value, 10).unwrap(),
             //     U256::from_str_radix(second_value, 10).unwrap(),
             // ];
             let expression = extract_raw_expression(comment.clone());

             ParserOutput::Instruction(Instruction { op: Op::ExpmodTraceGenSub, data, raw: expression })
         })),
        // domains[0] = point^trace_length - 1.
        // domains[1] = point^(trace_length / 2) - 1.
        (Regex::new(r"//\s*domains\[(\d+)]\s*=\s*(.+?)\s*-\s*(.+)").unwrap(),
         Box::new(|captures: &regex::Captures| {
             let index = &captures[1];
             let minuend = &captures[2];
             let subtrahend = &captures[3];
             let data = vec![index.to_string(), minuend.to_string(), subtrahend.to_string()];
             //     Data::U256(U256::from_str_radix(index, 10).unwrap()),
             //     Data::String(expression.to_string()),
             //     Data::U256(U256::from_str_radix(subtrahend, 10).unwrap()),
             // ];
             let expression = extract_raw_expression(comment.clone());

             ParserOutput::Instruction(Instruction { op: Op::Domain, data, raw: expression })
         })),
    ];

    for (regex, processor) in patterns {
        if let Some(captures) = regex.captures(&comment.clone()) {
            return processor(&captures);
        }
    }
    return ParserOutput::None;
}

fn extractor(parser_output: ParserOutput) {
    // match parser_output {
    //     ParserOutput::MemoryLayout(_, _) => {}
    //     ParserOutput::Instruction(ins) => {
    //         match ins.op {
    //             Op::Expmod => {
    //                 let mut map = HashMap::new();
    //                 map.insert("memory_address", "0x23c0");
    //                 map.insert("divisor", &ins.data[1].to_string());
    //                 // map
    //             }
    //         }
    //     }
    //     ParserOutput::None => {}
    // }
}

fn main() {
    let comments = reader("examples/cpu-constraint-poly/CpuConstraintPoly.sol");
    let mut context = Context::new();

    let mut solidity_data = SolidityData {
        memory_layout: HashMap::new(),
        memory: HashMap::new(),
        instructions: Vec::new(),
    };

    solidity_data.memory.insert("point".to_string(), "0x440".to_string());

    let mut memory_layout = Vec::new();

    for comment in comments {
        let parsed = parser(comment);
        match parsed {
            ParserOutput::Memory(des, mem) => {
                solidity_data.memory_layout.insert(des.clone(), mem.clone());
                let mut map: HashMap<&str, String> = HashMap::new();
                let MemoryRange { start, end } = mem;
                map.insert("start", format!("{:#x}", start));
                map.insert("end", format!("{:#x}", end));
                map.insert("description", des);
                memory_layout.push(map);
            }
            ParserOutput::Instruction(ins) => {
                solidity_data.instructions.push(ins);
            }
            ParserOutput::None => {}
        }
    }

    for mem in solidity_data.memory_layout.iter() {
        context.insert(mem.0, &mem.1.start);
    }
    let mut instruction_code = String::new();

    for ins in solidity_data.instructions {
        let mut template_name = "";
        match ins.op {
            Op::Expmod => {
                let Instruction { op, data, raw } = ins;
                let divisor = U256::from_str_radix(&data[1], 10).unwrap();
                // calculate the memory address
                let start = solidity_data.memory_layout.get("expmods").unwrap().start;
                let address = start + U256::from(32) * U256::from_str_radix(&data[0], 10).unwrap();
                //store the position, eg: point^(trace_length / 16) => 0x23c0
                solidity_data.memory.insert(raw.right, format!("{:#x}", address));
                context.insert("memory_address", &format!("{:#x}", address));
                context.insert("index", &data[0].to_string());
                context.insert("divisor", &divisor.to_string());
                template_name = "sol/expmod.sol.template";
            }
            Op::Expmod8 => {
                let Instruction { op, data, raw } = ins;
                // calculate the memory address
                let start = solidity_data.memory_layout.get("expmods").unwrap().start;
                let address = start + U256::from(32) * U256::from_str_radix(&data[0], 10).unwrap();
                solidity_data.memory.insert(raw.right, format!("{:#x}", address));
                context.insert("memory_address", &format!("{:#x}", address));
                context.insert("index", &data[0].to_string());
                template_name = "sol/expmod-8.sol.template";
            }
            Op::ExpmodTraceGen => {
                let Instruction { op, data, raw } = ins;
                let divisor = U256::from_str_radix(&data[1], 10).unwrap();
                // calculate the memory address
                let start = solidity_data.memory_layout.get("expmods").unwrap().start;
                let address = start + U256::from(32) * U256::from_str_radix(&data[0], 10).unwrap();
                solidity_data.memory.insert(raw.right, format!("{:#x}", address));
                context.insert("memory_address", &format!("{:#x}", address));
                context.insert("divisor", &divisor.to_string());
                context.insert("index", &data[0].to_string());
                template_name = "sol/expmod-trace-gen.sol.template";
            }
            Op::ExpmodTraceGenOdd => {
                let Instruction { op, data, raw } = ins;
                let mul = U256::from_str_radix(&data[1], 10).unwrap();
                let divisor = U256::from_str_radix(&data[2], 10).unwrap();
                // calculate the memory address
                let start = solidity_data.memory_layout.get("expmods").unwrap().start;
                let address = start + U256::from(32) * U256::from_str_radix(&data[0], 10).unwrap();
                solidity_data.memory.insert(raw.right, format!("{:#x}", address));
                context.insert("memory_address", &format!("{:#x}", address));
                context.insert("mul", &mul.to_string());
                context.insert("divisor", &divisor.to_string());
                context.insert("index", &data[0].to_string());
                template_name = "sol/expmod-trace-gen-odd.sol.template";
            }
            Op::ExpmodTraceGenSub => {
                let Instruction { op, data, raw } = ins;
                let subtrahend = U256::from_str_radix(&data[1], 10).unwrap();
                // calculate the memory address
                let start = solidity_data.memory_layout.get("expmods").unwrap().start;
                let address = start + U256::from(32) * U256::from_str_radix(&data[0], 10).unwrap();
                solidity_data.memory.insert(raw.right, format!("{:#x}", address));
                context.insert("memory_address", &format!("{:#x}", address));
                context.insert("subtrahend", &subtrahend.to_string());
                context.insert("index", &data[0].to_string());
                template_name = "sol/expmod-trace-gen-sub.sol.template";
            }
            Op::Domain => {
                let Instruction { op, data, raw } = ins;
                match data[0].as_str() {
                    "10" => {
                        template_name = "sol/domain-10.sol.template";
                    }
                    "12" => {
                        template_name = "sol/domain-12.sol.template";
                    }
                    "13" => {
                        template_name = "sol/domain-13.sol.template";
                    }
                    "14" => {
                        template_name = "sol/domain-14.sol.template";
                    }
                    _ => {
                        println!("{}", data[0]);
                        let minuend_expression = data[1].clone();
                        let minuend = solidity_data.memory.get(&minuend_expression).unwrap();
                        let subtrahend_expression = data[2].clone();
                        let subtrahend = match U256::from_str_radix(&subtrahend_expression, 10) {
                            Ok(v) => { &v.to_string() }
                            Err(_) => {
                                solidity_data.memory.get(&subtrahend_expression).unwrap()
                            }
                        };
                        // calculate the memory address
                        let start = solidity_data.memory_layout.get("domains").unwrap().start;
                        let address = start + U256::from(32) * U256::from_str_radix(&data[0], 10).unwrap();
                        context.insert("memory_address", &format!("{:#x}", address));
                        context.insert("minuend_expression", &minuend_expression);
                        context.insert("minuend", minuend);
                        context.insert("subtrahend_expression", &subtrahend_expression);
                        context.insert("subtrahend", &subtrahend);
                        context.insert("index", &data[0].to_string());
                        template_name = "sol/domain.sol.template";
                    }
                }
            }
        }

        instruction_code.push_str(&TEMPLATES.render(template_name, &context).unwrap());
        instruction_code.push_str("\n");
    }

    // for data in &expmods_special_data {
    //     println!("{:?}", data);
    // }

    context.insert("memory_layout", &memory_layout);
    context.insert("instructions", &instruction_code);

    match TEMPLATES.render("sol/a.sol.template", &context) {
        Ok(s) => {
            fs::create_dir_all("generated").expect("Failed to create directory");
            let mut file = File::create("generated/gen.move").expect("Failed to create Move file");
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