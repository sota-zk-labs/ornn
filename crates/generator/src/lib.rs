pub mod sol_generator;
pub mod template_generator;
pub mod move_generator;

use ast::{ASTNode};
use registry::Registry;

pub trait GeneratorInput {}

impl GeneratorInput for ASTNode {}

pub trait Generator<T>
where
    T: GeneratorInput,
{
    fn generate(ast: &T, registry: &Box<dyn Registry>) -> (String, usize);
}
