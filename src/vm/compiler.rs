use crate::ast::Node;
use crate::executor::RelType;
use crate::vm::opcode::OpCode;

#[derive(Default)]
pub struct Compiler {
    pub instructions: Vec<OpCode>,
    pub constants: Vec<RelType>,
    pub functions: std::collections::HashMap<String, usize>,
    pub locals: Vec<std::collections::HashMap<String, usize>>,
    pub current_local_count: usize,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            constants: Vec::new(),
            functions: std::collections::HashMap::new(),
            locals: Vec::new(),
            current_local_count: 0,
        }
    }

    /// Recursively flattens an AST math/logic tree into linear opcodes.
    /// Returns false if the node cannot be compiled (e.g. it contains side-effects or variables).
    pub fn compile_node(&mut self, node: &Node) -> bool {
        match node {
            Node::IntLiteral(v) => {
                let idx = self.add_constant(RelType::Int(*v));
                self.instructions.push(OpCode::Constant(idx));
                true
            }
            Node::FloatLiteral(v) => {
                let idx = self.add_constant(RelType::Float(*v));
                self.instructions.push(OpCode::Constant(idx));
                true
            }
            Node::StringLiteral(v) => {
                let idx = self.add_constant(RelType::Str(v.clone()));
                self.instructions.push(OpCode::Constant(idx));
                true
            }
            Node::BoolLiteral(v) => {
                let idx = self.add_constant(RelType::Bool(*v));
                self.instructions.push(OpCode::Constant(idx));
                true
            }
            Node::Add(l, r) => {
                if !self.compile_node(l) || !self.compile_node(r) { return false; }
                self.instructions.push(OpCode::Add);
                true
            }
            Node::Sub(l, r) => {
                if !self.compile_node(l) || !self.compile_node(r) { return false; }
                self.instructions.push(OpCode::Subtract);
                true
            }
            Node::Mul(l, r) => {
                if !self.compile_node(l) || !self.compile_node(r) { return false; }
                self.instructions.push(OpCode::Multiply);
                true
            }
            Node::Div(l, r) => {
                if !self.compile_node(l) || !self.compile_node(r) { return false; }
                self.instructions.push(OpCode::Divide);
                true
            }
            Node::Eq(l, r) => {
                if !self.compile_node(l) || !self.compile_node(r) { return false; }
                self.instructions.push(OpCode::Equal);
                true
            }
            Node::Gt(l, r) => {
                if !self.compile_node(l) || !self.compile_node(r) { return false; }
                self.instructions.push(OpCode::Greater);
                true
            }
            Node::Lt(l, r) => {
                if !self.compile_node(l) || !self.compile_node(r) { return false; }
                self.instructions.push(OpCode::Less);
                true
            }
            Node::Block(stmts) => {
                for stmt in stmts {
                    if !self.compile_node(stmt) { return false; }
                }
                true
            }
            Node::If(cond, then_block, else_block) => {
                if !self.compile_node(cond) { return false; }
                let jump_if_false_idx = self.instructions.len();
                self.instructions.push(OpCode::JumpIfFalse(0)); // Placeholder

                if !self.compile_node(then_block) { return false; }

                if let Some(else_branch) = else_block {
                    let jump_idx = self.instructions.len();
                    self.instructions.push(OpCode::Jump(0)); // Placeholder

                    // Backpatch JumpIfFalse to jump here (start of else block)
                    self.instructions[jump_if_false_idx] = OpCode::JumpIfFalse(self.instructions.len());

                    if !self.compile_node(else_branch) { return false; }

                    // Backpatch unconditional Jump to jump past the else block
                    self.instructions[jump_idx] = OpCode::Jump(self.instructions.len());
                } else {
                    // Backpatch JumpIfFalse to jump past the then block
                    self.instructions[jump_if_false_idx] = OpCode::JumpIfFalse(self.instructions.len());
                }
                true
            }
            Node::Assign(ident, expr) => {
                if !self.compile_node(expr) { return false; }
                
                if !self.locals.is_empty() {
                    let idx = if let Some(idx) = self.resolve_local(&ident) {
                        idx
                    } else {
                        // Declare new local
                        let idx = self.current_local_count;
                        self.locals.last_mut().unwrap().insert(ident.clone(), idx);
                        self.current_local_count += 1;
                        idx
                    };
                    self.instructions.push(OpCode::SetLocal(idx));
                } else {
                    let idx = self.add_constant(RelType::Str(ident.clone()));
                    self.instructions.push(OpCode::SetGlobal(idx));
                }
                true
            }
            Node::Identifier(ident) => {
                if !self.locals.is_empty() {
                    if let Some(idx) = self.resolve_local(&ident) {
                        self.instructions.push(OpCode::GetLocal(idx));
                        return true;
                    }
                }
                let idx = self.add_constant(RelType::Str(ident.clone()));
                self.instructions.push(OpCode::GetGlobal(idx));
                true
            }
            Node::Call(name, args) => {
                match name.as_str() {
                    "str_len" => {
                        if args.len() != 1 { return false; }
                        if !self.compile_node(&args[0]) { return false; }
                        self.instructions.push(OpCode::StringLength);
                        true
                    }
                    "str_contains" => {
                        if args.len() != 2 { return false; }
                        if !self.compile_node(&args[0]) { return false; } // String source
                        if !self.compile_node(&args[1]) { return false; } // Pattern / char class
                        self.instructions.push(OpCode::StringContainsChars);
                        true
                    }
                    "str_split" => {
                        if args.len() != 2 { return false; }
                        if !self.compile_node(&args[0]) { return false; } // Target String
                        if !self.compile_node(&args[1]) { return false; } // Delimiter
                        self.instructions.push(OpCode::StringSplit);
                        true
                    }
                    "arr_contains" => {
                        if args.len() != 2 { return false; }
                        if !self.compile_node(&args[0]) { return false; } // Array
                        if !self.compile_node(&args[1]) { return false; } // Search String
                        self.instructions.push(OpCode::ArrayContains);
                        true
                    }
                    "read_file" => {
                        if args.len() != 1 { return false; }
                        if !self.compile_node(&args[0]) { return false; }
                        self.instructions.push(OpCode::ReadFile);
                        true
                    }
                    _ => {
                        if let Some(&target_ip) = self.functions.get(name) {
                            for arg in args {
                                if !self.compile_node(arg) { return false; }
                            }
                            self.instructions.push(OpCode::Call(target_ip, args.len()));
                            true
                        } else {
                            // FFI ExternCall Fallback
                            // Compile arguments in normal order (left-to-right).
                            // At runtime, the top of the stack will be the last argument.
                            for arg in args {
                                if !self.compile_node(arg) { return false; }
                            }
                            let name_idx = self.add_constant(RelType::Str(name.clone()));
                            self.instructions.push(OpCode::ExternCall { name_idx, arg_count: args.len() });
                            true
                        }
                    }
                }
            }
            Node::Print(expr) => {
                if !self.compile_node(expr) { return false; }
                self.instructions.push(OpCode::Print);
                true
            }
            Node::Return(expr) => {
                if !self.compile_node(expr) { return false; }
                self.instructions.push(OpCode::Return);
                true
            }
            Node::FnDef(name, args, body) => {
                // Record jump to skip function body
                let jump_over_idx = self.instructions.len();
                self.instructions.push(OpCode::Jump(0));
                
                // Track start IP of the function
                let func_ip = self.instructions.len();
                self.functions.insert(name.clone(), func_ip);
                
                // Track execution context (isolate Local variables)
                self.locals.push(std::collections::HashMap::new());
                let mut previous_local_count = self.current_local_count;
                self.current_local_count = 0;
                
                // Assign numerical indices to explicit arguments contextually mapped against base_pointer
                for arg in args {
                    self.locals.last_mut().unwrap().insert(arg.clone(), self.current_local_count);
                    self.current_local_count += 1;
                }
                
                if !self.compile_node(body) { return false; }
                
                // Auto-return if body doesn't terminate with one explicitly
                self.instructions.push(OpCode::Return);
                
                // Restore outside context
                self.locals.pop();
                self.current_local_count = previous_local_count;
                
                // Backpatch Jump to skip execution block dynamically
                // Backpatch Jump to skip execution block dynamically
                self.instructions[jump_over_idx] = OpCode::Jump(self.instructions.len());
                true
            }
            Node::ObjectLiteral(map) => {
                self.instructions.push(OpCode::AllocateDict);
                for (k, v) in map {
                    let k_idx = self.add_constant(RelType::Str(k.clone()));
                    self.instructions.push(OpCode::Constant(k_idx)); // Push Key
                    if !self.compile_node(v) { return false; } // Push Value
                    self.instructions.push(OpCode::SetProperty); // Mutate and push Object reference
                }
                true
            }
            Node::PropertyGet(obj_node, prop_name) => {
                if !self.compile_node(obj_node) { return false; } // Push Object
                let k_idx = self.add_constant(RelType::Str(prop_name.clone()));
                self.instructions.push(OpCode::Constant(k_idx)); // Push Key
                self.instructions.push(OpCode::GetProperty); // Pushes Extracted Value
                true
            }
            Node::PropertySet(obj_node, prop_name, value_node) => {
                if !self.compile_node(obj_node) { return false; } // Push Object
                let k_idx = self.add_constant(RelType::Str(prop_name.clone()));
                self.instructions.push(OpCode::Constant(k_idx)); // Push Key
                if !self.compile_node(value_node) { return false; } // Push Value
                self.instructions.push(OpCode::SetProperty); // Pushes Modified Object
                true
            }
            _ => false,
        }
    }

    pub fn resolve_local(&self, name: &str) -> Option<usize> {
        for scope in self.locals.iter().rev() {
            if let Some(&idx) = scope.get(name) {
                return Some(idx);
            }
        }
        None
    }

    fn add_constant(&mut self, val: RelType) -> usize {
        if let Some(idx) = self.constants.iter().position(|c| c == &val) {
            return idx;
        }
        self.constants.push(val);
        self.constants.len() - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Node;

    #[test]
    fn test_compile_add() {
        let mut compiler = Compiler::new();
        let ast = Node::Add(
            Box::new(Node::IntLiteral(10)),
            Box::new(Node::IntLiteral(5)),
        );
        assert!(compiler.compile_node(&ast));
        
        assert_eq!(
            compiler.instructions,
            vec![OpCode::Constant(0), OpCode::Constant(1), OpCode::Add]
        );
        assert_eq!(compiler.constants, vec![RelType::Int(10), RelType::Int(5)]);
    }

    #[test]
    fn test_deduplicate_constants() {
        let mut compiler = Compiler::new();
        let ast = Node::Add(
            Box::new(Node::IntLiteral(10)),
            Box::new(Node::IntLiteral(10)),
        );
        assert!(compiler.compile_node(&ast));
        
        assert_eq!(
            compiler.instructions,
            vec![OpCode::Constant(0), OpCode::Constant(0), OpCode::Add]
        );
        assert_eq!(compiler.constants, vec![RelType::Int(10)]);
    }
}
