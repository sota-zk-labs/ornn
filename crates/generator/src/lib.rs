pub mod sol_generator;
pub mod template_generator;
pub mod move_generator;

use std::collections::HashMap;
use tera::Context;
use registry::Registry;
use structure::ast::ASTNode;

pub trait GeneratorInput {}
pub trait GeneratorOutput {}

impl GeneratorInput for ASTNode {}

pub trait Generator<I,O>
where
    I: GeneratorInput,
    O: GeneratorOutput,
{
    fn generate(&mut self, ast: &I) -> O;
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
