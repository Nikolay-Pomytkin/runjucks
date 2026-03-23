use crate::errors::Result;
use crate::{lexer, parser, renderer};
use serde_json::Value;

#[derive(Debug)]
pub struct Environment {
    pub autoescape: bool,
    pub dev: bool,
}

impl Default for Environment {
    fn default() -> Self {
        Self {
            autoescape: true,
            dev: false,
        }
    }
}

impl Environment {
    pub fn render_string(&self, template: String, context: Value) -> Result<String> {
        let tokens = lexer::tokenize(&template)?;
        let ast = parser::parse(&tokens)?;
        renderer::render(self, &ast, &context)
    }
}
