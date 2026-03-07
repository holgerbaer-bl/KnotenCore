use crate::executor::{ExecutionEngine, RelType, ExecResult, StackFrame};
use crate::ast::Node;
use std::collections::HashMap;

impl ExecutionEngine {
    pub fn evaluate(&mut self, node: &Node) -> ExecResult {
        let res = self.evaluate_inner(node);
        if let ExecResult::Fault(ref err) = res {
            if err.contains("Permission Denied") || err.contains("Sandbox") {
                self.permission_fault = Some(err.clone());
            }
        }
        res
    }

    pub fn evaluate_inner(&mut self, node: &Node) -> ExecResult {
        match node {
            Node::Identifier(name) => {
                if let Some(v) = self.get_var(name) { ExecResult::Value(v) }
                else { ExecResult::Fault(format!("Variable '{}' not found", name)) }
            }
            Node::IntLiteral(v) => ExecResult::Value(RelType::Int(*v)),
            Node::FloatLiteral(v) => ExecResult::Value(RelType::Float(*v)),
            Node::BoolLiteral(v) => ExecResult::Value(RelType::Bool(*v)),
            Node::Add(l, r) => self.do_math(l, '+', r),
            Node::Assign(name, expr) => {
                match self.evaluate_inner(expr) {
                    ExecResult::Value(v) => { self.set_var(name.clone(), v.clone()); ExecResult::Value(v) }
                    ExecResult::ReturnBlockInfo(v) => { self.set_var(name.clone(), v.clone()); ExecResult::Value(v) }
                    err => err,
                }
            }
            Node::While(cond, body) => {
                while let ExecResult::Value(RelType::Bool(true)) = self.evaluate_inner(cond) {
                    match self.evaluate_inner(body) {
                        ExecResult::Value(v) => self.release_handles(&v),
                        ExecResult::ReturnBlockInfo(v) => return ExecResult::ReturnBlockInfo(v),
                        ExecResult::Fault(e) => return ExecResult::Fault(e),
                    }
                }
                ExecResult::Value(RelType::Void)
            }
            Node::Block(nodes) => {
                let mut last_val = RelType::Void;
                let len = nodes.len();
                for (i, n) in nodes.iter().enumerate() {
                    match self.evaluate_inner(n) {
                        ExecResult::Value(v) => {
                            if i < len - 1 { self.release_handles(&v); }
                            else { last_val = v; }
                        }
                        ExecResult::ReturnBlockInfo(v) => return ExecResult::ReturnBlockInfo(v),
                        ExecResult::Fault(e) => return ExecResult::Fault(e),
                    }
                }
                ExecResult::Value(last_val)
            }
            
            Node::StringLiteral(v) => ExecResult::Value(RelType::Str(v.clone())),
            Node::Time | Node::GlobalTime => ExecResult::Value(RelType::Float(self.startup_time.elapsed().as_secs_f64())),
            
            Node::Sub(l, r) => self.do_math(l, '-', r),
            Node::Mul(l, r) => self.do_math(l, '*', r),
            Node::Div(l, r) => self.do_math(l, '/', r),

            Node::Eq(l, r) => self.do_compare(l, "==", r),
            Node::Lt(l, r) => self.do_compare(l, "<", r),
            Node::Gt(l, r) => self.do_compare(l, ">", r),

            Node::Print(expr) => {
                match self.evaluate_inner(expr) {
                    ExecResult::Value(v) => { println!("{}", v); ExecResult::Value(RelType::Void) }
                    err => err,
                }
            }
            Node::Sin(expr) => {
                match self.evaluate_inner(expr) {
                    ExecResult::Value(RelType::Float(v)) => ExecResult::Value(RelType::Float(v.sin())),
                    ExecResult::Value(_) => ExecResult::Fault("Sin expects float".into()),
                    err => err,
                }
            }
            Node::Cos(expr) => {
                match self.evaluate_inner(expr) {
                    ExecResult::Value(RelType::Float(v)) => ExecResult::Value(RelType::Float(v.cos())),
                    ExecResult::Value(_) => ExecResult::Fault("Cos expects float".into()),
                    err => err,
                }
            }
            Node::If(cond, then_b, else_b) => {
                match self.evaluate_inner(cond) {
                    ExecResult::Value(RelType::Bool(true)) => self.evaluate_inner(then_b),
                    ExecResult::Value(RelType::Bool(false)) => {
                        if let Some(eb) = else_b { self.evaluate_inner(eb) }
                        else { ExecResult::Value(RelType::Void) }
                    }
                    _ => ExecResult::Fault("If condition must be boolean".into()),
                }
            }

            Node::FnDef(name, params, body) => {
                self.set_var(name.clone(), RelType::FnDef(name.clone(), params.clone(), body.clone()));
                ExecResult::Value(RelType::Void)
            }

            Node::Call(name, args) => {
                let func = if let Some(f) = self.get_var(name) { f } else { return ExecResult::Fault(format!("Function '{}' not found", name)) };
                match func {
                    RelType::FnDef(_, params, body) => {
                        if params.len() != args.len() { return ExecResult::Fault(format!("'{}' expects {} args, got {}", name, params.len(), args.len())) }
                        let mut locals = HashMap::with_capacity(params.len());
                        for (p, a) in params.iter().zip(args.iter()) {
                            match self.evaluate_inner(a) {
                                ExecResult::Value(v) => { locals.insert(p.clone(), v); }
                                err => return err,
                            }
                        }
                        self.call_stack.push(StackFrame { locals });
                        let res = self.evaluate_inner(&body);
                        
                        // Clean up scope
                        if let Some(frame) = self.call_stack.pop() {
                            for (_, val) in frame.locals { self.release_handles(&val); }
                        }

                        match res {
                            ExecResult::ReturnBlockInfo(v) => ExecResult::Value(v),
                            other => other,
                        }
                    }
                    _ => ExecResult::Fault(format!("'{}' is not a function", name)),
                }
            }

            Node::Return(expr) => {
                let v = match self.evaluate_inner(&*expr) { ExecResult::Value(v) => v, err => return err };
                ExecResult::ReturnBlockInfo(v)
            }

            _ => self.evaluate_extra(node),
        }
    }

    pub fn do_math(&mut self, left: &Node, op: char, right: &Node) -> ExecResult {
        let lv = match self.evaluate_inner(left) { ExecResult::Value(v) => v, err => return err };
        let rv = match self.evaluate_inner(right) { ExecResult::Value(v) => v, err => return err };
        let res = match op {
            '+' => match (lv, rv) {
                (RelType::Int(a), RelType::Int(b)) => RelType::Int(a + b),
                (RelType::Float(a), RelType::Float(b)) => RelType::Float(a + b),
                (RelType::Str(a), RelType::Str(b)) => RelType::Str(a + &b),
                _ => return ExecResult::Fault("Invalid types for +".into()),
            },
            '-' => match (lv, rv) {
                (RelType::Int(a), RelType::Int(b)) => RelType::Int(a - b),
                (RelType::Float(a), RelType::Float(b)) => RelType::Float(a - b),
                _ => return ExecResult::Fault("Invalid types for -".into()),
            },
            '*' => match (lv, rv) {
                (RelType::Int(a), RelType::Int(b)) => RelType::Int(a * b),
                (RelType::Float(a), RelType::Float(b)) => RelType::Float(a * b),
                _ => return ExecResult::Fault("Invalid types for *".into()),
            },
            '/' => match (lv, rv) {
                (RelType::Int(a), RelType::Int(b)) => { if b == 0 { return ExecResult::Fault("Div by zero".into()) } RelType::Int(a / b) },
                (RelType::Float(a), RelType::Float(b)) => RelType::Float(a / b),
                _ => return ExecResult::Fault("Invalid types for /".into()),
            },
            _ => return ExecResult::Fault(format!("Unknown operator: {}", op)),
        };
        ExecResult::Value(res)
    }

    pub fn do_compare(&mut self, left: &Node, op: &str, right: &Node) -> ExecResult {
        let lv = match self.evaluate_inner(left) { ExecResult::Value(v) => v, err => return err };
        let rv = match self.evaluate_inner(right) { ExecResult::Value(v) => v, err => return err };
        let res = match op {
            "==" => RelType::Bool(lv == rv),
            "<" => match (lv, rv) {
                (RelType::Int(a), RelType::Int(b)) => RelType::Bool(a < b),
                (RelType::Float(a), RelType::Float(b)) => RelType::Bool(a < b),
                _ => return ExecResult::Fault("Invalid types for <".into()),
            },
            ">" => match (lv, rv) {
                (RelType::Int(a), RelType::Int(b)) => RelType::Bool(a > b),
                (RelType::Float(a), RelType::Float(b)) => RelType::Bool(a > b),
                _ => return ExecResult::Fault("Invalid types for >".into()),
            },
            _ => return ExecResult::Fault(format!("Unknown comparison: {}", op)),
        };
        ExecResult::Value(res)
    }
}
