pub mod ast_parser;

pub trait ParserOutput {}

pub trait Parser<T>
where
    T: ParserOutput,
{
    fn parse(expression: &str) -> T;
}
