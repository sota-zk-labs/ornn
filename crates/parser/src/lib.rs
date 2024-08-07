use std::collections::VecDeque;
use ast::ASTNode;

pub trait ParserOutput {}

pub trait Parser<T>
where
    T: ParserOutput,
{
    fn parse(expression: &str) -> T;
}

pub struct ASTParser {}

// Define the possible types of tokens
#[derive(Debug, Clone, PartialEq)]
enum Token {
    Expression(String),
    Operator(char),
    Parenthesis(char),
}

impl ParserOutput for ASTNode {}

impl Parser<ASTNode> for ASTParser {
    fn parse(expression: &str) -> ASTNode {
        let postfix_tokens = Self::shunting_yard(expression);
        Self::build_ast(postfix_tokens)
    }
}

impl ASTParser {
    fn build_ast(postfix_tokens: Vec<Token>) -> ASTNode {
        let mut stack = Vec::new();

        for token in postfix_tokens {
            match token {
                Token::Expression(expr) => stack.push(ASTNode::Variable(expr)),
                Token::Operator(op) => {
                    let right = stack.pop().expect("Invalid expression");
                    let left = stack.pop().expect("Invalid expression");
                    stack.push(ASTNode::Operation(Box::new(left), op, Box::new(right)));
                }
                _ => panic!("Unexpected token"),
            }
        }

        stack.pop().expect("Invalid expression")
    }

    fn precedence(op: char) -> i64 {
        match op {
            '+' | '-' => 1,
            '*' | '/' => 2,
            '^' => 3,
            _ => 0,
        }
    }

    fn tokenize(expression: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut chars = expression.chars().peekable();
        let mut current_expr = String::new();
        let mut in_expression = false;

        while let Some(ch) = chars.next() {
            match ch {
                'a'..='z' | 'A'..='Z' | '_' | '0'..='9' | '[' | ']' => {
                    current_expr.push(ch);
                    in_expression = true;
                }
                '^' | '+' | '-' | '*' | '/' => {
                    if in_expression {
                        tokens.push(Token::Expression(current_expr.clone()));
                        current_expr.clear();
                        in_expression = false;
                    }
                    tokens.push(Token::Operator(ch));
                }
                '(' | ')' => {
                    if in_expression {
                        tokens.push(Token::Expression(current_expr.clone()));
                        current_expr.clear();
                        in_expression = false;
                    }
                    tokens.push(Token::Parenthesis(ch));
                }
                ' ' => {
                    if in_expression {
                        tokens.push(Token::Expression(current_expr.clone()));
                        current_expr.clear();
                        in_expression = false;
                    }
                    continue;
                }
                _ => {
                    println!("Unexpected character: {}", ch);
                    panic!("Unexpected character")
                }
            }
        }

        if in_expression {
            tokens.push(Token::Expression(current_expr));
        }

        tokens
    }

    fn shunting_yard(expression: &str) -> Vec<Token> {
        let mut output = Vec::new();
        let mut operators = VecDeque::new();

        let tokens = Self::tokenize(expression); // Tokenize the input

        for token in tokens {
            match token {
                Token::Expression(_) => output.push(token),
                Token::Operator(op) => {
                    while let Some(&Token::Operator(top_op)) = operators.back() {
                        if Self::precedence(op) <= Self::precedence(top_op) {
                            output.push(operators.pop_back().unwrap());
                        } else {
                            break;
                        }
                    }
                    operators.push_back(token);
                }
                Token::Parenthesis('(') => operators.push_back(token),
                Token::Parenthesis(')') => {
                    while let Some(op) = operators.pop_back() {
                        if op == Token::Parenthesis('(') {
                            break;
                        }
                        output.push(op);
                    }
                }
                _ => {}
            }
        }

        while let Some(op) = operators.pop_back() {
            output.push(op);
        }

        output
    }
}

fn indent_to_storage_location(indent: usize) -> String {
    // Example mapping based on indent level; modify this as needed.
    format!("0x1d4{}", indent)
}

mod tests {
    use generator::{ASTGenerator, Generator};
    use crate::{ASTParser, Parser};

    #[test]
    fn test() {
        // let expression = "(point^(trace_length / 128) - trace_generator^(trace_length / 64)) * (point^(trace_length / 128) - trace_generator^(trace_length / 32)) * (point^(trace_length / 128) - trace_generator^(3 * trace_length / 64)) * (point^(trace_length / 128) - trace_generator^(trace_length / 16)) * (point^(trace_length / 128) - trace_generator^(5 * trace_length / 64)) * (point^(trace_length / 128) - trace_generator^(3 * trace_length / 32)) * (point^(trace_length / 128) - trace_generator^(7 * trace_length / 64)) * (point^(trace_length / 128) - trace_generator^(trace_length / 8)) * (point^(trace_length / 128) - trace_generator^(9 * trace_length / 64)) * (point^(trace_length / 128) - trace_generator^(5 * trace_length / 32)) * (point^(trace_length / 128) - trace_generator^(11 * trace_length / 64)) * (point^(trace_length / 128) - trace_generator^(3 * trace_length / 16)) * (point^(trace_length / 128) - trace_generator^(13 * trace_length / 64)) * (point^(trace_length / 128) - trace_generator^(7 * trace_length / 32)) * (point^(trace_length / 128) - trace_generator^(15 * trace_length / 64)) * domain8";
        // let expression = "point^(trace_length / 128) - trace_generator^(3 * trace_length / 4)";
        // let expression = "column0_row0 - (column0_row1 + column0_row1)";
        // let expression = " 1 - (cpu__decode__opcode_range_check__bit_2 + cpu__decode__opcode_range_check__bit_4 + cpu__decode__opcode_range_check__bit_3)";
        // let expression  = "column1_row0 + column1_row2 * 2 + column1_row4 * 4 + column1_row6 * 8 + column1_row8 * 18446744073709551616 + column1_row10 * 36893488147419103232 + column1_row12 * 73786976294838206464 + column1_row14 * 147573952589676412928";
        let expression = "domains[0]";
        let ast = ASTParser::parse(expression);
        // println!("AST: {:?}", ast);
        let solidity_code = ASTGenerator::generate(&ast);
        println!("{:?}", ast);
        println!("{}", solidity_code);
    }
}


