use std::collections::VecDeque;
use structure::ast::ASTNode;
use crate::{Parser, ParserOutput};

impl ParserOutput for ASTNode {}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Expression(String),
    Operator(String),
    Parenthesis(char),
    None,
}

pub struct ASTParser {}

impl Parser<ASTNode> for ASTParser {
    fn parse(expression: &str) -> ASTNode {
        let postfix_tokens = Self::shunting_yard(expression);
        Self::build_ast(postfix_tokens)
    }
}

impl ASTParser {
    fn build_ast(postfix_tokens: Vec<Token>) -> ASTNode {
        let mut stack = Vec::new();
        if postfix_tokens.len() == 0 {
            return ASTNode::Variable("### unknown ###".to_string().into());
        }

        for token in postfix_tokens {
            match token {
                Token::Expression(expr) => {
                    stack.push(ASTNode::Variable(expr.into()))
                }
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

    fn precedence(op: &str) -> i64 {
        match op {
            "+" | "-" => 1,
            "*" | "/" => 2,
            "^" => 3,
            "**" => 4,
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
                '+' | '-' | '*' | '/' | '^' => {
                    if in_expression {
                        tokens.push(Token::Expression(current_expr.clone()));
                        current_expr.clear();
                        in_expression = false;
                    }
                    if ch == '*' && chars.peek() == Some(&'*') {
                        chars.next(); // Consume the next '*'
                        tokens.push(Token::Operator("**".to_string()));
                    } else {
                        tokens.push(Token::Operator(ch.to_string()));
                    }
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
                    println!("### warn ### Unexpected character: {} in expression: {}", ch, expression);

                    return vec![Token::None];
                    // panic!("Unexpected character")
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
                Token::Operator(ref op) => {
                    while let Some(Token::Operator(ref top_op)) = operators.back() {
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use super::*;

    #[test]
    fn single_var() {
        let expression = "domains[0]";
        let ast = ASTParser::parse(expression);

        println!("structure: {}", ast);
    }

    #[test]
    fn simple_expression() {
        let expression = "column0_row0 - (column0_row1 + column0_row1)";
        let ast = ASTParser::parse(expression);

        println!("structure: {}", ast);
    }

    #[test]
    fn simple_registry() {
        let expression = "point^(trace_length / 128) - trace_generator^(3 * trace_length / 4)";
        let ast = ASTParser::parse(expression);

        println!("structure: {}", ast);
    }

    #[test]
    fn large_expression() {
        let expression = "column1_row0 + column1_row2 * 2 + column1_row4 * 4 + column1_row6 * 8 + column1_row8 * 18446744073709551616 + column1_row10 * 36893488147419103232 + column1_row12 * 73786976294838206464 + column1_row14 * 147573952589676412928";
        let ast = ASTParser::parse(expression);

        println!("structure: {}", ast);
    }

    #[test]
    fn parse_failed() {
        let expression = "d_{i}";
        let ast = ASTParser::parse(expression);

        println!("structure: {}", ast);
    }
}
