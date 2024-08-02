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
    ExpmodSpecial,
    ExpmodTraceGen,
}
struct Instruction {
    op: Op,
    data: Vec<U256>,
}
#[derive(Copy, Clone, Debug)]
struct MemoryRange {
    start: U256,
    end: U256,
}

struct SolidityData {
    memory_layout: HashMap<String, MemoryRange>,
    memory: HashMap<String, U256>,
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
        comments.push(single_comment.as_str().trim_end_matches('.').to_string());
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
             let data = vec![
                 U256::from_str_radix(first_value, 10).unwrap(),
                 U256::from_str_radix(second_value, 10).unwrap(),
             ];
             ParserOutput::Instruction(Instruction { op: Op::Expmod, data })
             // format!("// MOVE: expmods index: {}, divisor: {}", first_value, second_value)
         })),
        // expmods[8] = point^trace_length.
        (Regex::new(r"//\s*expmods\[(\d+)\]\s*=\s*point\^trace_length").unwrap(),
         Box::new(|captures: &regex::Captures| {
             let first_value = &captures[1];
             let data = vec![
                 U256::from_str_radix(first_value, 10).unwrap(),
             ];
             ParserOutput::Instruction(Instruction { op: Op::ExpmodSpecial, data })
         })),
        // expmods[9] = trace_generator^(trace_length / 64).
        (Regex::new(r"//\s*expmods\[(\d+)\]\s*=\s*trace_generator\^\(trace_length\s*/\s*(\d+)\)").unwrap(),
         Box::new(|captures: &regex::Captures| {
             let first_value = &captures[1];
             let second_value = &captures[2];
             let data = vec![
                 U256::from_str_radix(first_value, 10).unwrap(),
                 U256::from_str_radix(second_value, 10).unwrap(),
             ];
             ParserOutput::Instruction(Instruction { op: Op::ExpmodTraceGen, data })
         })),
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

fn main() {
    let comments = reader("examples/cpu-constraint-poly/CpuConstraintPoly.sol");
    let mut context = Context::new();

    let mut solidity_data = SolidityData {
        memory_layout: HashMap::new(),
        memory: HashMap::new(),
        instructions: Vec::new(),
    };

    let mut memory_layout = Vec::new();
    let mut expmods_data = Vec::new();
    let mut expmods_special_data = Vec::new();
    let mut expmods_trace_gen_data = Vec::new();

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
                let mut map: HashMap<&str, String> = HashMap::new();
                let Instruction { op, data, } = ins;
                let divisor = data[1];
                // calculate the memory address
                let start = solidity_data.memory_layout.get("expmods").unwrap().start;
                let address = start + U256::from(32) * data[0];
                context.insert("memory_address", &format!("{:#x}", address));
                context.insert("divisor", &divisor.to_string());
                // map.insert("memory_address", format!("{:#x}", address));
                // map.insert("divisor", divisor.to_string());
                expmods_data.push(map);
                template_name = "sol/expmod.sol.template";
            }
            Op::ExpmodSpecial => {
                let mut map: HashMap<&str, String> = HashMap::new();
                let Instruction { op, data, } = ins;
                // calculate the memory address
                let start = solidity_data.memory_layout.get("expmods").unwrap().start;
                let address = start + U256::from(32) * data[0];
                context.insert("memory_address", &format!("{:#x}", address));
                // map.insert("memory_address", format!("{:#x}", address));
                expmods_special_data.push(map);
                template_name = "sol/expmod-special.sol.template";

            }
            Op::ExpmodTraceGen => {
                let mut map: HashMap<&str, String> = HashMap::new();
                let Instruction { op, data, } = ins;
                let divisor = data[1];
                // calculate the memory address
                let start = solidity_data.memory_layout.get("expmods").unwrap().start;
                let address = start + U256::from(32) * data[0];
                context.insert("memory_address", &format!("{:#x}", address));
                context.insert("divisor", &divisor.to_string());
                // map.insert("memory_address", format!("{:#x}", address));
                // map.insert("divisor", divisor.to_string());
                expmods_trace_gen_data.push(map);
                template_name = "sol/expmod-trace-gen.sol.template";

            }
        }

        instruction_code.push_str(&TEMPLATES.render(template_name, &context).unwrap());
        instruction_code.push_str("\n");
    }

    // for data in &expmods_special_data {
    //     println!("{:?}", data);
    // }

    context.insert("memory_layout", &memory_layout);

    context.insert("expmods", &expmods_data);
    context.insert("expmods_special", &expmods_special_data);


    context.insert("username", &"Bob");
    context.insert("numbers", &vec![1, 2, 3]);
    context.insert("show_all", &false);
    context.insert("bio", &"<script>alert('pwnd');</script>");

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