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
    Expmod
}
struct Instruction {
    op: Op,
    data: Vec<U256>,
}
struct Memory {
    start: U256,
    end: U256,
    description: String,
}


struct SolidityData {
    memory_layout: HashMap<String, String>,
    memory: HashMap<String, U256>,
    instructions: Vec<Instruction>,
}

enum ParserOutput {
    Memory(Memory),
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
    for single_comment in single_line_re.find_iter(&sol_code) {
        comments.push(single_comment.as_str().to_string());
    }
    comments
}

fn parser(comment: String) -> ParserOutput {
    let patterns: Vec<(Regex, Box<dyn Fn(&regex::Captures) -> ParserOutput>)> = vec![
        // [0x0, 0x20) - periodic_column/pedersen/points/x.
        (Regex::new(r"\[(0x[0-9a-fA-F]+),\s*(0x[0-9a-fA-F]+)\)\s*-\s*(.+)").unwrap(),
         Box::new(|captures: &regex::Captures| {
             let start_value = &captures[1];
             let end_value = &captures[2];
             let description = &captures[3].trim();
             ParserOutput::Memory(Memory {
                 start: U256::from_str_radix(start_value, 16).unwrap(),
                 end: U256::from_str_radix(end_value, 16).unwrap(),
                 description: description.to_string(),
             })
             // format!("// MOVE: start: {}, end: {}, description: {}", start_value, end_value, description)
         })),
        // expmods[0] = point^(trace_length / 2048).
        (Regex::new(r"//\s*expmods\[(\d+)\]\s*=\s*point\^\(trace_length\s*/\s*(\d+)\)").unwrap(),
         Box::new(|captures: &regex::Captures| {
             let first_value = &captures[1];
             let second_value = &captures[2];
             let data = vec![
                 U256::from_str_radix(first_value, 10).unwrap(),
                 U256::from_str_radix(second_value, 10).unwrap(),
             ];
             ParserOutput::Instruction(Instruction { op: Op::Expmod, data })
             // format!("// MOVE: expmods index: {}, divisor: {}", first_value, second_value)
         })),
        //expmods[9] = trace_generator^(trace_length / 64).
    ];

    for (regex, processor) in patterns {
        if let Some(captures) = regex.captures(&comment) {
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

// fn calculate_memory() -> {
//
// }
fn main() {
    let comments = reader("examples/cpu-constraint-poly/CpuConstraintPoly.sol");

    let mut memory_layout = Vec::new();
    let mut expmods_data = Vec::new();

    for comment in comments {
        let parsed = parser(comment);
        match parsed {
            ParserOutput::Memory(mem) => {
                let mut map: HashMap<&str, String> = HashMap::new();
                let Memory { start, end, description } = mem;
                map.insert("start", format!("{:#x}", start));
                map.insert("end", format!("{:#x}", end));
                map.insert("description", description);
                memory_layout.push(map);
            }
            ParserOutput::Instruction(ins) => {
                match ins.op {
                    Op::Expmod => {
                        let mut map: HashMap<&str, String> = HashMap::new();
                        let Instruction { op, data, } = ins;
                        let divisor = data[1];
                        println!("memory_address: 0x23c0, divisor: {}", divisor);
                        map.insert("memory_address", "0x23c0".parse().unwrap());
                        map.insert("divisor", divisor.to_string());
                        expmods_data.push(map);
                    }
                }
            }
            ParserOutput::None => {}
        }
    }

    let mut context = Context::new();

    context.insert("expmods", &expmods_data);
    context.insert("memory_layout", &memory_layout);


    context.insert("username", &"Bob");
    context.insert("numbers", &vec![1, 2, 3]);
    context.insert("show_all", &false);
    context.insert("bio", &"<script>alert('pwnd');</script>");

    // A one off template
    Tera::one_off("hello", &Context::new(), true).unwrap();

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