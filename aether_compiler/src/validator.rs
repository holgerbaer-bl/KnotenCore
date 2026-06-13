use knoten_core_types::ast::Node;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

pub struct Validator {
    pub errors: Vec<String>,
    import_stack: HashSet<String>,
    struct_registry: std::collections::HashMap<String, Vec<(String, knoten_core_types::ast::Type)>>,
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            import_stack: HashSet::new(),
            struct_registry: std::collections::HashMap::new(),
        }
    }

    pub fn validate(&mut self, node: &Node) -> Result<(), Vec<String>> {
        self.errors.clear();
        self.import_stack.clear();
        self.check_node(node);
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    fn check_node(&mut self, node: &Node) {
        match node {
            Node::Assign(name, val) => {
                if name.is_empty() {
                    self.errors
                        .push("Assign: Identifier name cannot be empty".to_string());
                }
                self.check_node(val);
                if is_string_literal(val)
                    && (name.ends_with("_int")
                        || name.ends_with("_i")
                        || name.starts_with("num_")
                        || name.starts_with("int_"))
                {
                    self.errors.push(format!(
                        "ERR_STATIC_TYPE_MISMATCH: Cannot assign StringLiteral to numerically declared variable '{}'",
                        name
                    ));
                }
            }
            Node::Store { key, value } => {
                if key.is_empty() {
                    self.errors.push("Store: Key cannot be empty".to_string());
                }
                self.check_node(value);
            }
            Node::Load { key } => {
                if key.is_empty() {
                    self.errors.push("Load: Key cannot be empty".to_string());
                }
            }
            Node::Add(l, r)
            | Node::Sub(l, r)
            | Node::Mul(l, r)
            | Node::Div(l, r)
            | Node::Mat4Mul(l, r)
            | Node::Eq(l, r)
            | Node::Lt(l, r)
            | Node::Gt(l, r)
            | Node::Lte(l, r)
            | Node::Gte(l, r)
            | Node::NotEq(l, r)
            | Node::And(l, r)
            | Node::Or(l, r)
            | Node::Concat(l, r)
            | Node::BitAnd(l, r)
            | Node::BitShiftLeft(l, r)
            | Node::BitShiftRight(l, r)
            | Node::LoadSample(l, r) => {
                self.check_node(l);
                self.check_node(r);
            }
            Node::Not(n) => {
                self.check_node(n);
            }
            Node::Fetch {
                method: _,
                url: _,
                callback,
            } => {
                self.check_node(callback);
            }
            Node::Extract { source, path } => {
                self.check_node(source);
                self.check_node(path);
            }
            Node::ObjectLiteral(map) => {
                for v in map.values() {
                    self.check_node(v);
                }
            }
            Node::PropertyGet(obj, _) => {
                self.check_node(obj);
            }
            Node::PropertySet(obj, _, val) => {
                self.check_node(obj);
                self.check_node(val);
            }
            Node::ArrayGet(arr, idx) => {
                self.check_node(arr);
                self.check_node(idx);
                if !is_integral_node(idx) {
                    self.errors.push(
                        "ArrayGet: index must be an integer expression (IntLiteral)".to_string(),
                    );
                }
            }
            Node::ArraySet(arr, idx, val) => {
                self.check_node(arr);
                self.check_node(idx);
                self.check_node(val);
                if !is_integral_node(idx) {
                    self.errors.push(
                        "ArraySet: index must be an integer expression (IntLiteral)".to_string(),
                    );
                }
            }
            Node::ArrayPush(arr, val) => {
                self.check_node(arr);
                self.check_node(val);
            }
            Node::ArrayLen(arr) => {
                self.check_node(arr);
            }
            Node::MapGet(map, key) | Node::MapHasKey(map, key) => {
                self.check_node(map);
                self.check_node(key);
            }
            Node::MapSet(map, key, val) => {
                self.check_node(map);
                self.check_node(key);
                self.check_node(val);
            }
            Node::Sin(n)
            | Node::Cos(n)
            | Node::FileRead(n)
            | Node::FSRead(n)
            | Node::Print(n)
            | Node::EvalJSONNative(n)
            | Node::ToString(n)
            | Node::LoadShader(n)
            | Node::PollEvents(n)
            | Node::PlayAudioFile(n)
            | Node::LoadMesh(n)
            | Node::LoadTexture(n)
            | Node::LoadFont(n)
            | Node::UILabel(n)
            | Node::UIButton(n)
            | Node::UITextInput(n)
            | Node::Return(n)
            | Node::Abs(n) => {
                self.check_node(n);
            }
            Node::FileWrite(f, d) | Node::FSWrite(f, d) => {
                self.check_node(f);
                self.check_node(d);
            }
            Node::FnDef(name, params, body) => {
                if name.is_empty() {
                    self.errors
                        .push("FnDef: Function name cannot be empty".to_string());
                }
                for param in params {
                    if param.is_empty() {
                        self.errors
                            .push(format!("FnDef ({}): Parameter name cannot be empty", name));
                    }
                }
                self.check_node(body);
            }
            Node::Call(name, args) | Node::NativeCall(name, args) => {
                if name.is_empty() {
                    self.errors
                        .push("Call/NativeCall: Function name cannot be empty".to_string());
                }
                for arg in args {
                    self.check_node(arg);
                }
            }
            Node::ExternCall {
                module,
                function,
                args,
            } => {
                if module.is_empty() || function.is_empty() {
                    self.errors
                        .push("ExternCall: Module and function cannot be empty".to_string());
                }
                for arg in args {
                    self.check_node(arg);
                }
            }
            Node::Block(nodes) | Node::ArrayCreate(nodes) => {
                for n in nodes {
                    self.check_node(n);
                }
            }
            Node::If(cond, then_b, else_b) => {
                self.check_node(cond);
                self.check_node(then_b);
                if let Some(eb) = else_b {
                    self.check_node(eb);
                }
            }
            Node::While(cond, body) => {
                self.check_node(cond);
                self.check_node(body);
            }
            Node::Import(path) => {
                if !Path::new(path).exists() {
                    self.errors
                        .push(format!("Import: File does not exist: {}", path));
                } else {
                    // Simple circular import check
                    if self.import_stack.contains(path) {
                        self.errors
                            .push(format!("Import: Circular dependency detected: {}", path));
                        return;
                    }

                    self.import_stack.insert(path.clone());
                    match fs::read_to_string(path) {
                        Ok(source) => {
                            let parsed_result = if path.ends_with(".nod") {
                                serde_json::from_str::<Node>(&source)
                                    .map_err(|e| format!("JSON Parse Error: {}", e))
                            } else {
                                // Parse as .knoten
                                let mut parser = crate::parser::Parser::new(&source);
                                parser.parse().map_err(|e| format!("Parser Error: {:?}", e))
                            };
                            match parsed_result {
                                Ok(parsed) => self.check_node(&parsed),
                                Err(e) => self.errors.push(format!("Import ({}): {}", path, e)),
                            }
                        }
                        Err(e) => self
                            .errors
                            .push(format!("Import ({}): File Read Error: {}", path, e)),
                    }
                    self.import_stack.remove(path);
                }
            }
            Node::Index(target, idx) => {
                self.check_node(target);
                self.check_node(idx);
            }
            Node::RenderMesh(s, v, m) | Node::PlaySample(s, v, m) | Node::InitWindow(s, v, m) => {
                self.check_node(s);
                self.check_node(v);
                self.check_node(m);
            }
            Node::PlayNote(c, f, d, w) => {
                self.check_node(c);
                self.check_node(f);
                self.check_node(d);
                self.check_node(w);
            }
            Node::Time | Node::GlobalTime => {}
            Node::RenderAsset(s, m, t, u) => {
                self.check_node(s);
                self.check_node(m);
                self.check_node(t);
                self.check_node(u);
            }
            Node::UIWindow(_, title, body) => {
                self.check_node(title);
                self.check_node(body);
            }
            Node::UISetStyle(r, s, a, f, bi, bh) => {
                self.check_node(r);
                self.check_node(s);
                self.check_node(a);
                self.check_node(f);
                if let Some(n) = bi {
                    self.check_node(n);
                }
                if let Some(n) = bh {
                    self.check_node(n);
                }
            }
            Node::UIHorizontal(b)
            | Node::UIFullscreen(b)
            | Node::UIGrid(_, _, b)
            | Node::UIScrollArea(_, b) => {
                self.check_node(b);
            }
            Node::UIHBox(nodes) | Node::UIVBox(nodes) => {
                for n in nodes {
                    self.check_node(n);
                }
            }
            Node::UISplitPanel {
                factor,
                left_body,
                right_body,
                ..
            } => {
                self.check_node(factor);
                self.check_node(left_body);
                self.check_node(right_body);
                if let Node::FloatLiteral(f) = &**factor
                    && !(0.0..=1.0).contains(f)
                {
                    self.errors.push(format!(
                        "ERR_INVALID_LAYOUT_FACTOR: factor {} out of range (0.0..=1.0)",
                        f
                    ));
                }
            }
            Node::DrawText(t, x, y, s, c) => {
                self.check_node(t);
                self.check_node(x);
                self.check_node(y);
                self.check_node(s);
                self.check_node(c);
            }
            // Literals & Constants
            Node::IntLiteral(_)
            | Node::FloatLiteral(_)
            | Node::BoolLiteral(_)
            | Node::StringLiteral(_)
            | Node::Identifier(_)
            | Node::MapCreate
            | Node::InitGraphics
            | Node::InitAudio
            | Node::GetLastKeypress
            | Node::UIFillParent
            | Node::StopNote(_) => {}
            Node::SpawnIsolate { .. } => {}
            Node::DrawRect {
                x,
                y,
                width,
                height,
                color,
            } => {
                self.check_node(x);
                self.check_node(y);
                self.check_node(width);
                self.check_node(height);
                self.check_node(color);
            }
            Node::UIFixed {
                width,
                height,
                body,
            } => {
                self.check_node(width);
                self.check_node(height);
                self.check_node(body);
            }
            // Sprint 68: Native 3D/2D Render Scene Graph
            Node::RenderCanvas { body } => {
                self.check_node(body);
            }
            Node::Transform2D {
                x,
                y,
                rotation,
                scale,
                body,
            } => {
                self.check_node(x);
                self.check_node(y);
                self.check_node(rotation);
                self.check_node(scale);
                self.check_node(body);
            }
            Node::Sprite2D {
                texture_id,
                transform,
            } => {
                self.check_node(texture_id);
                self.check_node(transform);
            }
            Node::Camera3D {
                pos_x,
                pos_y,
                pos_z,
                target_x,
                target_y,
                target_z,
                fov,
            } => {
                self.check_node(pos_x);
                self.check_node(pos_y);
                self.check_node(pos_z);
                self.check_node(target_x);
                self.check_node(target_y);
                self.check_node(target_z);
                self.check_node(fov);
            }
            Node::Mesh3D {
                primitive,
                material,
            } => {
                self.check_node(primitive);
                self.check_node(material);
            }
            Node::Material3D {
                r,
                g,
                b,
                a,
                metallic,
                roughness,
                texture_id,
            } => {
                self.check_node(r);
                self.check_node(g);
                self.check_node(b);
                self.check_node(a);
                self.check_node(metallic);
                self.check_node(roughness);
                if let Some(tid) = texture_id {
                    self.check_node(tid);
                }
            }
            Node::PointLight3D {
                x,
                y,
                z,
                r,
                g,
                b,
                intensity,
            } => {
                self.check_node(x);
                self.check_node(y);
                self.check_node(z);
                self.check_node(r);
                self.check_node(g);
                self.check_node(b);
                self.check_node(intensity);
            }
            Node::MeshInstance3D {
                mesh_id,
                transform,
                color_offset,
                pbr,
            } => {
                self.check_node(mesh_id);
                self.check_node(transform);
                self.check_node(color_offset);
                self.check_node(pbr);
            }
            Node::FPSCamera { fov } => {
                self.check_node(fov);
            }
            Node::MouseGrab { enabled } => {
                self.check_node(enabled);
            }
            Node::RaycastSimple => {}
            Node::WeaponViewModel { mesh, tex } => {
                self.check_node(mesh);
                self.check_node(tex);
            }
            Node::CheckCollision {
                a_min,
                a_max,
                b_min,
                b_max,
            } => {
                self.check_node(a_min);
                self.check_node(a_max);
                self.check_node(b_min);
                self.check_node(b_max);
            }
            Node::AddWorldAABB { min, max } => {
                self.check_node(min);
                self.check_node(max);
            }
            Node::LoadComputeShader(val) => {
                self.check_node(val);
            }
            Node::DispatchCompute {
                shader_id,
                x,
                y,
                z,
                inputs,
            } => {
                self.check_node(shader_id);
                self.check_node(x);
                self.check_node(y);
                self.check_node(z);
                for n in inputs {
                    self.check_node(n);
                }
            }
            Node::DispatchComputeLoop {
                shader_id,
                iterations,
                inputs,
                matrix_handle,
            } => {
                self.check_node(shader_id);
                self.check_node(iterations);
                for n in inputs {
                    self.check_node(n);
                }
                if let Some(mh) = matrix_handle {
                    self.check_node(mh);
                }
            }
            Node::Modulo(l, r) => {
                self.check_node(l);
                self.check_node(r);
            }
            Node::Neg(expr) => {
                self.check_node(expr);
            }
            Node::StructDef { name, fields } => {
                if name.is_empty() {
                    self.errors
                        .push("StructDef: struct name cannot be empty".to_string());
                }
                if fields.is_empty() {
                    self.errors
                        .push(format!("StructDef: struct '{}' has no fields", name));
                }
                self.struct_registry.insert(name.clone(), fields.clone());
                for (field_name, _) in fields {
                    if field_name.is_empty() {
                        self.errors
                            .push(format!("StructDef '{}': field name cannot be empty", name));
                    }
                }
            }
            Node::StructCreate {
                struct_name,
                values,
            } => {
                let fields = self.struct_registry.get(struct_name).cloned();
                if let Some(fields) = fields {
                    if values.len() != fields.len() {
                        self.errors.push(format!(
                            "ERR_STRUCT_LAYOUT_MISMATCH: struct '{}' expects {} fields, got {}",
                            struct_name,
                            fields.len(),
                            values.len()
                        ));
                    } else {
                        for (i, (field_name, expected_type)) in fields.iter().enumerate() {
                            let actual = &values[i];
                            self.check_node(actual);
                            if !type_matches_node(expected_type, actual) {
                                self.errors.push(format!(
                                    "ERR_STRUCT_LAYOUT_MISMATCH: struct '{}' field '{}' expects {:?}, got incompatible value",
                                    struct_name, field_name, expected_type
                                ));
                            }
                        }
                    }
                } else {
                    self.errors.push(format!(
                        "StructCreate: unknown struct type '{}'",
                        struct_name
                    ));
                }
            }
            Node::StructFieldSet { obj, value, .. } => {
                self.check_node(obj);
                self.check_node(value);
            }
        }
    }
}

fn is_integral_node(node: &Node) -> bool {
    match node {
        Node::IntLiteral(_) => true,
        Node::Add(a, b) | Node::Sub(a, b) | Node::Mul(a, b) => {
            is_integral_node(a) && is_integral_node(b)
        }
        Node::Neg(inner) => is_integral_node(inner),
        _ => false,
    }
}

fn is_string_literal(node: &Node) -> bool {
    matches!(node, Node::StringLiteral(_))
}

fn type_matches_node(expected: &knoten_core_types::ast::Type, node: &Node) -> bool {
    match expected {
        knoten_core_types::ast::Type::Int => matches!(node, Node::IntLiteral(_)),
        knoten_core_types::ast::Type::Float => {
            matches!(node, Node::FloatLiteral(_) | Node::IntLiteral(_))
        }
        knoten_core_types::ast::Type::Bool => matches!(node, Node::BoolLiteral(_)),
        knoten_core_types::ast::Type::String => matches!(node, Node::StringLiteral(_)),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frontend_strict_type_mismatch() {
        let mut validator = Validator::new();
        let ast = Node::Assign(
            "int_value".to_string(),
            Box::new(Node::StringLiteral("hello".to_string())),
        );
        let result = validator.validate(&ast);
        assert!(result.is_err(), "String→int assign must trigger error");
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.contains("ERR_STATIC_TYPE_MISMATCH")),
            "Must contain type mismatch error"
        );
    }

    #[test]
    fn test_frontend_struct_layout_validation() {
        use knoten_core_types::ast::Type;
        let mut validator = Validator::new();
        let particle = Node::StructDef {
            name: "Particle".to_string(),
            fields: vec![
                ("id".to_string(), Type::Int),
                ("pos".to_string(), Type::Float),
            ],
        };
        validator.validate(&particle).unwrap();

        let create_ok = Node::StructCreate {
            struct_name: "Particle".to_string(),
            values: vec![Node::IntLiteral(1), Node::FloatLiteral(2.0)],
        };
        assert!(validator.validate(&create_ok).is_ok());

        let create_bad_type = Node::StructCreate {
            struct_name: "Particle".to_string(),
            values: vec![
                Node::StringLiteral("x".to_string()),
                Node::FloatLiteral(2.0),
            ],
        };
        let result = validator.validate(&create_bad_type);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.contains("ERR_STRUCT_LAYOUT_MISMATCH")),
            "String→Int field must trigger layout mismatch"
        );

        let create_bad_arity = Node::StructCreate {
            struct_name: "Particle".to_string(),
            values: vec![Node::IntLiteral(1)],
        };
        let result = validator.validate(&create_bad_arity);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .iter()
                .any(|e| e.contains("ERR_STRUCT_LAYOUT_MISMATCH"))
        );
    }

    #[test]
    fn test_frontend_split_panel_bounds() {
        let mut validator = Validator::new();
        let invalid = Node::UISplitPanel {
            direction: "Horizontal".to_string(),
            factor: Box::new(Node::FloatLiteral(1.5)),
            left_body: Box::new(Node::Block(vec![])),
            right_body: Box::new(Node::Block(vec![])),
        };
        let result = validator.validate(&invalid);
        assert!(result.is_err(), "Factor 1.5 must trigger error");
        assert!(
            result
                .unwrap_err()
                .iter()
                .any(|e| e.contains("ERR_INVALID_LAYOUT_FACTOR"))
        );

        let valid = Node::UISplitPanel {
            direction: "Vertical".to_string(),
            factor: Box::new(Node::FloatLiteral(0.3)),
            left_body: Box::new(Node::Block(vec![])),
            right_body: Box::new(Node::Block(vec![])),
        };
        assert!(validator.validate(&valid).is_ok());
    }
}
