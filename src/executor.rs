use crate::ast::Node;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum RelType {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Void,
}

impl std::fmt::Display for RelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RelType::Int(v) => write!(f, "{} (i64)", v),
            RelType::Float(v) => write!(f, "{:?} (f64)", v), // Using Debug to avoid dropping .0 on integers formatting like floats
            RelType::Bool(v) => write!(f, "{} (bool)", v),
            RelType::Str(v) => write!(f, "\"{}\" (String)", v),
            RelType::Void => write!(f, "void"),
        }
    }
}

pub struct ExecutionEngine {
    pub memory: HashMap<String, RelType>,
}

pub enum ExecResult {
    Value(RelType),
    ReturnBlockInfo(RelType), // Explicit return triggered
    Fault(String),
}

impl ExecutionEngine {
    pub fn new() -> Self {
        Self {
            memory: HashMap::new(),
        }
    }

    pub fn execute(&mut self, root: &Node) -> String {
        self.memory.clear();
        let res = self.evaluate(root);

        let mut out = String::new();
        match res {
            ExecResult::Value(val) | ExecResult::ReturnBlockInfo(val) => {
                out.push_str(&format!("Return: {}", val));
            }
            ExecResult::Fault(err) => {
                // Return exactly "Fault: ..." as tests expect it
                return format!("Fault: {}", err);
            }
        }

        if !self.memory.is_empty() {
            let mut keys: Vec<&String> = self.memory.keys().collect();
            // Deterministic state output order is important, albeit tests don't strictly assert the var sequence format,
            // they do exact equality of string matching on simple cases.
            // Better to sort just in case. However, some tests define order implicitly:
            // "Return: 42 (i64), Memory: x = 42, y = 42" implies sequential matching or loose containing.
            // Let's defer sorting and match the specific structure if we can.
            // We'll see how tests fail.
            out.push_str(", Memory: ");

            // To ensure 100% deterministic test behavior, sort variables.
            keys.sort();
            let mem_str: Vec<String> = keys
                .iter()
                .map(|k| {
                    let v = self.memory.get(*k).unwrap();
                    match v {
                        RelType::Str(s) => format!("{} = \"{}\"", k, s),
                        RelType::Float(f) => format!("{} = {:?}", k, f),
                        _ => format!(
                            "{} = {}",
                            k,
                            match v {
                                RelType::Int(i) => i.to_string(),
                                RelType::Bool(b) => b.to_string(),
                                _ => unreachable!(),
                            }
                        ),
                    }
                })
                .collect();

            out.push_str(&mem_str.join(", "));
        }

        out
    }

    fn evaluate(&mut self, node: &Node) -> ExecResult {
        match node {
            // Literals
            Node::IntLiteral(v) => ExecResult::Value(RelType::Int(*v)),
            Node::FloatLiteral(v) => ExecResult::Value(RelType::Float(*v)),
            Node::BoolLiteral(v) => ExecResult::Value(RelType::Bool(*v)),
            Node::StringLiteral(v) => ExecResult::Value(RelType::Str(v.clone())),

            // Mem
            Node::Identifier(name) => {
                if let Some(val) = self.memory.get(name) {
                    ExecResult::Value(val.clone())
                } else {
                    ExecResult::Fault("Undefined identifier".to_string())
                }
            }
            Node::Assign(name, expr_node) => match self.evaluate(expr_node) {
                ExecResult::Value(val) => {
                    self.memory.insert(name.clone(), val.clone());
                    ExecResult::Value(val)
                }
                ExecResult::ReturnBlockInfo(val) => {
                    self.memory.insert(name.clone(), val.clone());
                    ExecResult::Value(val)
                }
                fault => fault,
            },

            // Math
            Node::Add(l, r) => self.do_math(l, r, '+'),
            Node::Sub(l, r) => self.do_math(l, r, '-'),
            Node::Mul(l, r) => self.do_math(l, r, '*'),
            Node::Div(l, r) => self.do_math(l, r, '/'),

            // Logic
            Node::Eq(l, r) => {
                let lv = self.evaluate(l);
                let rv = self.evaluate(r);
                match (lv, rv) {
                    (ExecResult::Value(l_val), ExecResult::Value(r_val)) => {
                        ExecResult::Value(RelType::Bool(l_val == r_val))
                    }
                    (ExecResult::Fault(err), _) | (_, ExecResult::Fault(err)) => {
                        ExecResult::Fault(err)
                    }
                    _ => ExecResult::Fault("Invalid Eq semantics".to_string()),
                }
            }
            Node::Lt(l, r) => {
                let lv = self.evaluate(l);
                let rv = self.evaluate(r);
                match (lv, rv) {
                    (ExecResult::Value(RelType::Int(li)), ExecResult::Value(RelType::Int(ri))) => {
                        ExecResult::Value(RelType::Bool(li < ri))
                    }
                    (
                        ExecResult::Value(RelType::Float(lf)),
                        ExecResult::Value(RelType::Float(rf)),
                    ) => ExecResult::Value(RelType::Bool(lf < rf)),
                    (ExecResult::Fault(err), _) | (_, ExecResult::Fault(err)) => {
                        ExecResult::Fault(err)
                    }
                    _ => ExecResult::Fault("Invalid Lt semantics".to_string()),
                }
            }

            // Flow
            Node::If(cond, then_br, else_br) => {
                let cv = self.evaluate(cond);
                match cv {
                    ExecResult::Value(RelType::Bool(true)) => self.evaluate(then_br),
                    ExecResult::Value(RelType::Bool(false)) => {
                        if let Some(eb) = else_br {
                            self.evaluate(eb)
                        } else {
                            ExecResult::Value(RelType::Void)
                        }
                    }
                    ExecResult::Fault(err) => ExecResult::Fault(err),
                    _ => ExecResult::Fault("If condition not a boolean".to_string()),
                }
            }
            Node::While(cond, body) => {
                loop {
                    match self.evaluate(cond) {
                        ExecResult::Value(RelType::Bool(true)) => match self.evaluate(body) {
                            ExecResult::ReturnBlockInfo(r) => {
                                return ExecResult::ReturnBlockInfo(r);
                            }
                            ExecResult::Fault(err) => return ExecResult::Fault(err),
                            _ => {}
                        },
                        ExecResult::Value(RelType::Bool(false)) => break,
                        ExecResult::Fault(err) => return ExecResult::Fault(err),
                        _ => return ExecResult::Fault("While condition not a boolean".to_string()),
                    }
                }
                ExecResult::Value(RelType::Void) // while evaluate returns void naturally unless return hits
            }
            Node::Block(nodes) => {
                let mut last_val = RelType::Void;
                for n in nodes {
                    match self.evaluate(n) {
                        ExecResult::ReturnBlockInfo(val) => {
                            return ExecResult::ReturnBlockInfo(val);
                        }
                        ExecResult::Fault(err) => return ExecResult::Fault(err),
                        ExecResult::Value(val) => {
                            last_val = val;
                        }
                    }
                }
                ExecResult::Value(last_val)
            }
            Node::Return(val_node) => match self.evaluate(val_node) {
                ExecResult::Value(v) => ExecResult::ReturnBlockInfo(v),
                fault => fault,
            },
        }
    }

    fn do_math(&mut self, l: &Node, r: &Node, op: char) -> ExecResult {
        let lv = self.evaluate(l);
        let rv = self.evaluate(r);

        match (lv, rv) {
            (ExecResult::Value(RelType::Int(li)), ExecResult::Value(RelType::Int(ri))) => {
                match op {
                    '+' => ExecResult::Value(RelType::Int(li + ri)),
                    '-' => ExecResult::Value(RelType::Int(li - ri)),
                    '*' => ExecResult::Value(RelType::Int(li * ri)),
                    '/' => {
                        if ri == 0 {
                            ExecResult::Fault("Division by zero".to_string())
                        } else {
                            ExecResult::Value(RelType::Int(li / ri))
                        }
                    }
                    _ => unreachable!(),
                }
            }
            (ExecResult::Value(RelType::Float(lf)), ExecResult::Value(RelType::Float(rf))) => {
                match op {
                    '+' => ExecResult::Value(RelType::Float(lf + rf)),
                    '-' => ExecResult::Value(RelType::Float(lf - rf)),
                    '*' => ExecResult::Value(RelType::Float(lf * rf)),
                    '/' => {
                        if rf == 0.0 {
                            ExecResult::Fault("Division by zero".to_string())
                        } else {
                            ExecResult::Value(RelType::Float(lf / rf))
                        }
                    }
                    _ => unreachable!(),
                }
            }
            (ExecResult::Fault(err), _) | (_, ExecResult::Fault(err)) => ExecResult::Fault(err),
            _ => ExecResult::Fault("Mathematical type mismatch".to_string()),
        }
    }
}
