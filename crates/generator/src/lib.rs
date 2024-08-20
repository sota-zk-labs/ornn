pub mod sol_generator;
pub mod template_generator;
pub mod move_generator;

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
