// Sprint 273: JIT Multi-Pass Shader Graph Synthesis
// Compiles AST math chains into WGSL compute shader source code.

use knoten_core_types::ast::Node;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

pub struct ShaderGraphCompiler {
    cache: HashMap<u64, String>,
}

impl Default for ShaderGraphCompiler {
    fn default() -> Self {
        Self::new()
    }
}

impl ShaderGraphCompiler {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    pub fn compile(&mut self, node: &Node) -> String {
        let hash = structural_hash(node);
        if let Some(cached) = self.cache.get(&hash) {
            return cached.clone();
        }
        let expr = compile_expr(node);
        let source = format!(
            "@group(0) @binding(0) var<storage, read_write> data: array<f32>;\n\
             @compute @workgroup_size(64)\n\
             fn main(@builtin(global_invocation_id) id: vec3<u32>) {{\n\
                 let idx = id.x;\n\
                 if (idx >= arrayLength(&data)) {{\n\
                     return;\n\
                 }}\n\
                 let x = data[idx];\n\
                 let result = {expr};\n\
                 data[idx] = result;\n\
             }}\n",
            expr = expr
        );
        self.cache.insert(hash, source.clone());
        source
    }
}

fn structural_hash(node: &Node) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hash_node(node, &mut hasher);
    hasher.finish()
}

fn hash_node<H: std::hash::Hasher>(node: &Node, hasher: &mut H) {
    match node {
        Node::IntLiteral(v) => {
            0u8.hash(hasher);
            v.hash(hasher);
        }
        Node::FloatLiteral(v) => {
            1u8.hash(hasher);
            v.to_bits().hash(hasher);
        }
        Node::Add(l, r) => {
            2u8.hash(hasher);
            hash_node(l, hasher);
            hash_node(r, hasher);
        }
        Node::Sub(l, r) => {
            3u8.hash(hasher);
            hash_node(l, hasher);
            hash_node(r, hasher);
        }
        Node::Mul(l, r) => {
            4u8.hash(hasher);
            hash_node(l, hasher);
            hash_node(r, hasher);
        }
        Node::Div(l, r) => {
            5u8.hash(hasher);
            hash_node(l, hasher);
            hash_node(r, hasher);
        }
        Node::Neg(n) => {
            6u8.hash(hasher);
            hash_node(n, hasher);
        }
        Node::Sin(n) => {
            7u8.hash(hasher);
            hash_node(n, hasher);
        }
        Node::Cos(n) => {
            8u8.hash(hasher);
            hash_node(n, hasher);
        }
        _ => {
            255u8.hash(hasher);
        }
    }
}

fn compile_expr(node: &Node) -> String {
    match node {
        Node::FloatLiteral(v) => format!("{:.6}f", v),
        Node::IntLiteral(v) => format!("{:.6}f", *v as f64),
        Node::Add(l, r) => format!("({} + {})", compile_expr(l), compile_expr(r)),
        Node::Sub(l, r) => format!("({} - {})", compile_expr(l), compile_expr(r)),
        Node::Mul(l, r) => format!("({} * {})", compile_expr(l), compile_expr(r)),
        Node::Div(l, r) => format!("({} / {})", compile_expr(l), compile_expr(r)),
        Node::Neg(n) => format!("(-{})", compile_expr(n)),
        Node::Sin(n) => format!("sin({})", compile_expr(n)),
        Node::Cos(n) => format!("cos({})", compile_expr(n)),
        _ => "0.0".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jit_shader_graph_synthesis() {
        let ast = Node::Add(
            Box::new(Node::Mul(
                Box::new(Node::IntLiteral(2)),
                Box::new(Node::FloatLiteral(3.0)),
            )),
            Box::new(Node::FloatLiteral(5.0)),
        );
        let mut compiler = ShaderGraphCompiler::new();
        let source = compiler.compile(&ast);
        assert!(source.contains("@group(0) @binding(0)"));
        assert!(source.contains("@compute @workgroup_size(64)"));
        assert!(source.contains("(2.000000f * 3.000000f) + 5.000000f"));

        let source2 = compiler.compile(&ast);
        assert_eq!(source, source2, "Same AST must return cached result");
    }
}
