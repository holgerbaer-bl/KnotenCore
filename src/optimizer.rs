use crate::ast::Node;

pub fn count_nodes(node: &Node) -> usize {
    let mut count = 1;
    match node {
        Node::IntLiteral(_)
        | Node::FloatLiteral(_)
        | Node::BoolLiteral(_)
        | Node::StringLiteral(_)
        | Node::Identifier(_)
        | Node::InitGraphics
        | Node::InitAudio
        | Node::GetLastKeypress
        | Node::MapCreate
        | Node::Load { .. }
        | Node::Import(_) => {}
        Node::Add(l, r)
        | Node::Sub(l, r)
        | Node::Mul(l, r)
        | Node::Div(l, r)
        | Node::Modulo(l, r)
        | Node::Mat4Mul(l, r)
        | Node::Eq(l, r)
        | Node::Lt(l, r)
        | Node::Gt(l, r)
        | Node::BitAnd(l, r)
        | Node::BitShiftLeft(l, r)
        | Node::BitShiftRight(l, r)
        | Node::Concat(l, r) => {
            count += count_nodes(l) + count_nodes(r);
        }
        Node::Lte(l, r)
        | Node::Gte(l, r)
        | Node::NotEq(l, r)
        | Node::And(l, r)
        | Node::Or(l, r) => {
            count += count_nodes(l) + count_nodes(r);
        }
        Node::Not(n) => {
            count += count_nodes(n);
        }
        Node::Sin(n) | Node::Cos(n) | Node::Abs(n) | Node::Neg(n) => {
            count += count_nodes(n);
        }
        Node::Time | Node::GlobalTime => {}
        Node::Index(l, r)
        | Node::ArrayPush(l, r)
        | Node::ArrayGet(l, r)
        | Node::MapGet(l, r)
        | Node::MapHasKey(l, r)
        | Node::FileWrite(l, r)
        | Node::FSWrite(l, r)
        | Node::LoadSample(l, r) => {
            count += count_nodes(l) + count_nodes(r);
        }

        Node::Assign(_, val)
        | Node::Store { value: val, .. }
        | Node::ArrayLen(val)
        | Node::Print(val)
        | Node::EvalJSONNative(val)
        | Node::ToString(val)
        | Node::LoadShader(val)
        | Node::PollEvents(val)
        | Node::PropertyGet(val, _)
        | Node::PropertySet(_, _, val)
        | Node::StopNote(val)
        | Node::LoadMesh(val)
        | Node::LoadTexture(val)
        | Node::PlayAudioFile(val)
        | Node::LoadFont(val)
        | Node::UILabel(val)
        | Node::UIButton(val)
        | Node::UITextInput(val)
        | Node::FileRead(val)
        | Node::FSRead(val)
        | Node::Return(val) => {
            count += count_nodes(val);
        }

        Node::If(cond, then_b, else_b) => {
            count += count_nodes(cond) + count_nodes(then_b);
            if let Some(eb) = else_b {
                count += count_nodes(eb);
            }
        }
        Node::UIWindow(_, title, body) => {
            // New
            count += count_nodes(title);
            count += count_nodes(body);
        }
        Node::While(cond, body) => {
            count += count_nodes(cond) + count_nodes(body);
        }
        Node::Block(nodes)
        | Node::ArrayCreate(nodes)
        | Node::Call(_, nodes)
        | Node::NativeCall(_, nodes) => {
            for n in nodes {
                count += count_nodes(n);
            }
        }
        Node::ObjectLiteral(map) => {
            for v in map.values() {
                count += count_nodes(v);
            }
        }
        Node::ExternCall {
            module: _,
            function: _,
            args,
        } => {
            for n in args {
                count += count_nodes(n);
            }
        }
        Node::FnDef(_, _, body) => {
            count += count_nodes(body);
        }
        Node::InitWindow(w, h, t)
        | Node::RenderMesh(w, h, t)
        | Node::PlayNote(w, h, t)
        | Node::PlaySample(w, h, t) => {
            count += count_nodes(w) + count_nodes(h) + count_nodes(t);
        }
        Node::RenderAsset(a, b, c, d) => {
            count += count_nodes(a) + count_nodes(b) + count_nodes(c) + count_nodes(d);
        }
        Node::UISetStyle(a, b, c, d, opt_e, opt_f) => {
            count += count_nodes(a) + count_nodes(b) + count_nodes(c) + count_nodes(d);
            if let Some(e) = opt_e {
                count += count_nodes(e);
            }
            if let Some(f) = opt_f {
                count += count_nodes(f);
            }
        }
        Node::UIHorizontal(b)
        | Node::UIFullscreen(b)
        | Node::UIGrid(_, _, b)
        | Node::UIScrollArea(_, b) => {
            count += count_nodes(b);
        }
        Node::UIHBox(nodes) | Node::UIVBox(nodes) => {
            for n in nodes {
                count += count_nodes(n);
            }
        }
        Node::ArraySet(a, b, c) | Node::MapSet(a, b, c) => {
            count += count_nodes(a) + count_nodes(b) + count_nodes(c);
        }
        Node::DrawText(a, b, c, d, e) => {
            count +=
                count_nodes(a) + count_nodes(b) + count_nodes(c) + count_nodes(d) + count_nodes(e);
        }
        Node::Fetch {
            method: _,
            url: _,
            callback,
        } => {
            count += count_nodes(callback);
        }
        Node::Extract { source, path } => {
            count += count_nodes(source) + count_nodes(path);
        }
        Node::DrawRect {
            x,
            y,
            width,
            height,
            color,
        } => {
            count += count_nodes(x)
                + count_nodes(y)
                + count_nodes(width)
                + count_nodes(height)
                + count_nodes(color);
        }
        Node::UIFixed {
            width,
            height,
            body,
        } => {
            count += count_nodes(width) + count_nodes(height) + count_nodes(body);
        }
        Node::UIFillParent => {}
        // Sprint 68: Native 3D/2D Render Scene Graph
        Node::RenderCanvas { body } => {
            count += count_nodes(body);
        }
        Node::Transform2D {
            x,
            y,
            rotation,
            scale,
            body,
        } => {
            count += count_nodes(x)
                + count_nodes(y)
                + count_nodes(rotation)
                + count_nodes(scale)
                + count_nodes(body);
        }
        Node::Sprite2D {
            texture_id,
            transform,
        } => {
            count += count_nodes(texture_id) + count_nodes(transform);
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
            count += count_nodes(pos_x)
                + count_nodes(pos_y)
                + count_nodes(pos_z)
                + count_nodes(target_x)
                + count_nodes(target_y)
                + count_nodes(target_z)
                + count_nodes(fov);
        }
        Node::Mesh3D {
            primitive,
            material,
        } => {
            count += count_nodes(primitive) + count_nodes(material);
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
            count += count_nodes(r)
                + count_nodes(g)
                + count_nodes(b)
                + count_nodes(a)
                + count_nodes(metallic)
                + count_nodes(roughness);
            if let Some(tid) = texture_id {
                count += count_nodes(tid);
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
            count += count_nodes(x)
                + count_nodes(y)
                + count_nodes(z)
                + count_nodes(r)
                + count_nodes(g)
                + count_nodes(b)
                + count_nodes(intensity);
        }
        Node::MeshInstance3D {
            mesh_id,
            transform,
            color_offset,
            pbr,
        } => {
            count += count_nodes(mesh_id)
                + count_nodes(transform)
                + count_nodes(color_offset)
                + count_nodes(pbr);
        }
        Node::FPSCamera { fov } => {
            count += count_nodes(fov);
        }
        Node::MouseGrab { enabled } => {
            count += count_nodes(enabled);
        }
        Node::RaycastSimple => {}
        Node::WeaponViewModel { mesh, tex } => {
            count += count_nodes(mesh) + count_nodes(tex);
        }
        Node::CheckCollision {
            a_min,
            a_max,
            b_min,
            b_max,
        } => {
            count +=
                count_nodes(a_min) + count_nodes(a_max) + count_nodes(b_min) + count_nodes(b_max);
        }
        Node::AddWorldAABB { min, max } => {
            count += count_nodes(min) + count_nodes(max);
        }
        Node::LoadComputeShader(val) => {
            count += count_nodes(val);
        }
        Node::DispatchCompute {
            shader_id,
            x,
            y,
            z,
            inputs,
        } => {
            count += count_nodes(shader_id) + count_nodes(x) + count_nodes(y) + count_nodes(z);
            for n in inputs {
                count += count_nodes(n);
            }
        }
    }
    count
}

pub fn optimize(node: Node) -> Node {
    match node {
        Node::IntLiteral(v) => Node::IntLiteral(v),
        Node::FloatLiteral(v) => Node::FloatLiteral(v),
        Node::BoolLiteral(v) => Node::BoolLiteral(v),
        Node::StringLiteral(v) => Node::StringLiteral(v),
        Node::Identifier(name) => Node::Identifier(name),
        Node::Import(path) => Node::Import(path),
        Node::InitGraphics => Node::InitGraphics,
        Node::InitAudio => Node::InitAudio,
        Node::GetLastKeypress => Node::GetLastKeypress,
        Node::Fetch {
            method,
            url,
            callback,
        } => Node::Fetch {
            method,
            url,
            callback: Box::new(optimize(*callback)),
        },
        Node::Extract { source, path } => Node::Extract {
            source: Box::new(optimize(*source)),
            path: Box::new(optimize(*path)),
        },

        // Math Folding
        Node::Add(l, r) => optimize_math_op(*l, *r, '+'),
        Node::Sub(l, r) => optimize_math_op(*l, *r, '-'),
        Node::Mul(l, r) => optimize_math_op(*l, *r, '*'),
        Node::Div(l, r) => optimize_math_op(*l, *r, '/'),

        // Logic Folding
        Node::Eq(l, r) => optimize_eq(*l, *r),
        Node::Lt(l, r) => optimize_lt(*l, *r),
        Node::Gt(l, r) => optimize_gt(*l, *r),
        Node::Lte(l, r) => Node::Lte(Box::new(optimize(*l)), Box::new(optimize(*r))),
        Node::Gte(l, r) => Node::Gte(Box::new(optimize(*l)), Box::new(optimize(*r))),
        Node::NotEq(l, r) => Node::NotEq(Box::new(optimize(*l)), Box::new(optimize(*r))),
        Node::And(l, r) => Node::And(Box::new(optimize(*l)), Box::new(optimize(*r))),
        Node::Or(l, r) => Node::Or(Box::new(optimize(*l)), Box::new(optimize(*r))),
        Node::Not(n) => Node::Not(Box::new(optimize(*n))),

        // Bitwise Folding
        Node::BitAnd(l, r) => optimize_bitwise(*l, *r, '&'),
        Node::BitShiftLeft(l, r) => optimize_bitwise(*l, *r, '<'),
        Node::BitShiftRight(l, r) => optimize_bitwise(*l, *r, '>'),

        // Dead Code Elimination
        Node::If(cond, then_branch, else_branch) => {
            let opt_cond = optimize(*cond);
            match opt_cond {
                Node::BoolLiteral(true) => optimize(*then_branch),
                Node::BoolLiteral(false) => {
                    if let Some(eb) = else_branch {
                        optimize(*eb)
                    } else {
                        Node::Block(vec![])
                    }
                }
                _ => Node::If(
                    Box::new(opt_cond),
                    Box::new(optimize(*then_branch)),
                    else_branch.map(|eb| Box::new(optimize(*eb))),
                ),
            }
        }
        Node::While(cond, body) => {
            let opt_cond = optimize(*cond);
            match opt_cond {
                Node::BoolLiteral(false) => Node::Block(vec![]),
                _ => Node::While(Box::new(opt_cond), Box::new(optimize(*body))),
            }
        }
        Node::Block(nodes) => {
            let opt_nodes: Vec<Node> = nodes.into_iter().map(optimize).collect();
            Node::Block(opt_nodes)
        }

        // Standard Traversals
        Node::FnDef(name, params, body) => Node::FnDef(name, params, Box::new(optimize(*body))),
        Node::Call(name, args) => Node::Call(name, args.into_iter().map(optimize).collect()),
        Node::NativeCall(name, args) => {
            // Sprint 194: Try inlining before recursive optimization
            if let Some(inlined) = try_inline_native(&name, &args) {
                return optimize(inlined);
            }
            Node::NativeCall(name, args.into_iter().map(optimize).collect())
        }
        Node::ExternCall {
            module,
            function,
            args,
        } => Node::ExternCall {
            module,
            function,
            args: args.into_iter().map(optimize).collect(),
        },

        Node::Assign(name, val) => Node::Assign(name, Box::new(optimize(*val))),
        Node::Store { key, value } => Node::Store {
            key,
            value: Box::new(optimize(*value)),
        },
        Node::Load { key } => Node::Load { key },
        Node::ArrayCreate(nodes) => Node::ArrayCreate(nodes.into_iter().map(optimize).collect()),
        Node::ArrayGet(arr, index) => {
            let opt_arr = optimize(*arr);
            let opt_idx = optimize(*index);
            // Sprint 194: Fold ArrayGet(ArrayCreate(elems), IntLiteral(i)) at compile time
            if let (Node::ArrayCreate(elems), Node::IntLiteral(i)) = (&opt_arr, &opt_idx) {
                let idx = *i as usize;
                if idx < elems.len() {
                    return elems[idx].clone();
                }
                return Node::StringLiteral("".into()); // Void-equivalent for OOB
            }
            Node::ArrayGet(Box::new(opt_arr), Box::new(opt_idx))
        }
        Node::ArraySet(arr, index, val) => Node::ArraySet(
            Box::new(optimize(*arr)),
            Box::new(optimize(*index)),
            Box::new(optimize(*val)),
        ),
        Node::ArrayPush(arr, val) => {
            Node::ArrayPush(Box::new(optimize(*arr)), Box::new(optimize(*val)))
        }
        Node::ArrayLen(arr) => Node::ArrayLen(Box::new(optimize(*arr))),
        Node::MapCreate => Node::MapCreate,
        Node::MapGet(m, k) => Node::MapGet(Box::new(optimize(*m)), Box::new(optimize(*k))),
        Node::MapSet(m, k, v) => Node::MapSet(
            Box::new(optimize(*m)),
            Box::new(optimize(*k)),
            Box::new(optimize(*v)),
        ),
        Node::MapHasKey(m, k) => Node::MapHasKey(Box::new(optimize(*m)), Box::new(optimize(*k))),
        Node::UIWindow(id, title, block) => {
            // Modified
            Node::UIWindow(id, Box::new(optimize(*title)), Box::new(optimize(*block)))
        }
        Node::Index(arr, index) => {
            Node::Index(Box::new(optimize(*arr)), Box::new(optimize(*index)))
        }
        Node::Concat(l, r) => Node::Concat(Box::new(optimize(*l)), Box::new(optimize(*r))),

        Node::ObjectLiteral(map) => {
            let mut opt_map = std::collections::HashMap::new();
            for (k, v) in map {
                opt_map.insert(k, optimize(v));
            }
            Node::ObjectLiteral(opt_map)
        }
        Node::PropertyGet(obj, prop) => Node::PropertyGet(Box::new(optimize(*obj)), prop),
        Node::PropertySet(obj, prop, val) => {
            Node::PropertySet(Box::new(optimize(*obj)), prop, Box::new(optimize(*val)))
        }

        Node::Return(val) => Node::Return(Box::new(optimize(*val))),
        Node::Sin(n) => Node::Sin(Box::new(optimize(*n))),
        Node::Cos(n) => Node::Cos(Box::new(optimize(*n))),
        Node::Abs(n) => Node::Abs(Box::new(optimize(*n))),

        Node::Mat4Mul(l, r) => Node::Mat4Mul(Box::new(optimize(*l)), Box::new(optimize(*r))),
        Node::Time => Node::Time,
        Node::GlobalTime => Node::GlobalTime,
        Node::FileRead(f) => Node::FileRead(Box::new(optimize(*f))), // Modified
        Node::FSRead(f) => Node::FSRead(Box::new(optimize(*f))),     // New
        Node::FileWrite(f, d) => Node::FileWrite(Box::new(optimize(*f)), Box::new(optimize(*d))), // Modified
        Node::FSWrite(f, d) => Node::FSWrite(Box::new(optimize(*f)), Box::new(optimize(*d))), // New
        Node::Print(val) => Node::Print(Box::new(optimize(*val))),
        Node::EvalJSONNative(val) => Node::EvalJSONNative(Box::new(optimize(*val))),
        Node::ToString(val) => Node::ToString(Box::new(optimize(*val))),

        Node::InitWindow(w, h, t) => Node::InitWindow(
            Box::new(optimize(*w)),
            Box::new(optimize(*h)),
            Box::new(optimize(*t)),
        ),
        Node::LoadShader(val) => Node::LoadShader(Box::new(optimize(*val))),
        Node::RenderMesh(s, v, m) => Node::RenderMesh(
            Box::new(optimize(*s)),
            Box::new(optimize(*v)),
            Box::new(optimize(*m)),
        ),
        Node::PollEvents(body) => Node::PollEvents(Box::new(optimize(*body))),

        Node::PlayNote(c, f, w) => Node::PlayNote(
            Box::new(optimize(*c)),
            Box::new(optimize(*f)),
            Box::new(optimize(*w)),
        ),
        Node::StopNote(c) => Node::StopNote(Box::new(optimize(*c))),

        Node::LoadMesh(p) => Node::LoadMesh(Box::new(optimize(*p))),
        Node::LoadTexture(p) => Node::LoadTexture(Box::new(optimize(*p))),
        Node::PlayAudioFile(p) => Node::PlayAudioFile(Box::new(optimize(*p))),
        Node::RenderAsset(s, m, t, u) => Node::RenderAsset(
            Box::new(optimize(*s)),
            Box::new(optimize(*m)),
            Box::new(optimize(*t)),
            Box::new(optimize(*u)),
        ),

        Node::LoadFont(p) => Node::LoadFont(Box::new(optimize(*p))),
        Node::DrawText(t, x, y, s, c) => Node::DrawText(
            Box::new(optimize(*t)),
            Box::new(optimize(*x)),
            Box::new(optimize(*y)),
            Box::new(optimize(*s)),
            Box::new(optimize(*c)),
        ),
        Node::UILabel(t) => Node::UILabel(Box::new(optimize(*t))),
        Node::UIButton(t) => Node::UIButton(Box::new(optimize(*t))),
        Node::UITextInput(v) => Node::UITextInput(Box::new(optimize(*v))),
        Node::UISetStyle(r, s, a, f, bi, bh) => Node::UISetStyle(
            Box::new(optimize(*r)),
            Box::new(optimize(*s)),
            Box::new(optimize(*a)),
            Box::new(optimize(*f)),
            bi.map(|n| Box::new(optimize(*n))),
            bh.map(|n| Box::new(optimize(*n))),
        ),
        Node::UIHorizontal(b) => Node::UIHorizontal(Box::new(optimize(*b))),
        Node::UIHBox(nodes) => Node::UIHBox(nodes.into_iter().map(optimize).collect()),
        Node::UIVBox(nodes) => Node::UIVBox(nodes.into_iter().map(optimize).collect()),
        Node::UIFullscreen(b) => Node::UIFullscreen(Box::new(optimize(*b))),
        Node::UIGrid(cols, id, body) => Node::UIGrid(cols, id, Box::new(optimize(*body))),
        Node::UIScrollArea(id, body) => Node::UIScrollArea(id, Box::new(optimize(*body))),
        Node::LoadSample(id, p) => {
            Node::LoadSample(Box::new(optimize(*id)), Box::new(optimize(*p)))
        }
        Node::PlaySample(id, v, p) => Node::PlaySample(
            Box::new(optimize(*id)),
            Box::new(optimize(*v)),
            Box::new(optimize(*p)),
        ),
        Node::DrawRect {
            x,
            y,
            width,
            height,
            color,
        } => Node::DrawRect {
            x: Box::new(optimize(*x)),
            y: Box::new(optimize(*y)),
            width: Box::new(optimize(*width)),
            height: Box::new(optimize(*height)),
            color: Box::new(optimize(*color)),
        },
        Node::UIFixed {
            width,
            height,
            body,
        } => Node::UIFixed {
            width: Box::new(optimize(*width)),
            height: Box::new(optimize(*height)),
            body: Box::new(optimize(*body)),
        },
        Node::UIFillParent => Node::UIFillParent,
        // Sprint 68: Native 3D/2D Render Scene Graph
        Node::RenderCanvas { body } => Node::RenderCanvas {
            body: Box::new(optimize(*body)),
        },
        Node::Transform2D {
            x,
            y,
            rotation,
            scale,
            body,
        } => Node::Transform2D {
            x: Box::new(optimize(*x)),
            y: Box::new(optimize(*y)),
            rotation: Box::new(optimize(*rotation)),
            scale: Box::new(optimize(*scale)),
            body: Box::new(optimize(*body)),
        },
        Node::Sprite2D {
            texture_id,
            transform,
        } => Node::Sprite2D {
            texture_id: Box::new(optimize(*texture_id)),
            transform: Box::new(optimize(*transform)),
        },
        Node::Camera3D {
            pos_x,
            pos_y,
            pos_z,
            target_x,
            target_y,
            target_z,
            fov,
        } => Node::Camera3D {
            pos_x: Box::new(optimize(*pos_x)),
            pos_y: Box::new(optimize(*pos_y)),
            pos_z: Box::new(optimize(*pos_z)),
            target_x: Box::new(optimize(*target_x)),
            target_y: Box::new(optimize(*target_y)),
            target_z: Box::new(optimize(*target_z)),
            fov: Box::new(optimize(*fov)),
        },
        Node::Mesh3D {
            primitive,
            material,
        } => Node::Mesh3D {
            primitive: Box::new(optimize(*primitive)),
            material: Box::new(optimize(*material)),
        },
        Node::Material3D {
            r,
            g,
            b,
            a,
            metallic,
            roughness,
            texture_id,
        } => Node::Material3D {
            r: Box::new(optimize(*r)),
            g: Box::new(optimize(*g)),
            b: Box::new(optimize(*b)),
            a: Box::new(optimize(*a)),
            metallic: Box::new(optimize(*metallic)),
            roughness: Box::new(optimize(*roughness)),
            texture_id: texture_id.map(|t| Box::new(optimize(*t))),
        },
        Node::PointLight3D {
            x,
            y,
            z,
            r,
            g,
            b,
            intensity,
        } => Node::PointLight3D {
            x: Box::new(optimize(*x)),
            y: Box::new(optimize(*y)),
            z: Box::new(optimize(*z)),
            r: Box::new(optimize(*r)),
            g: Box::new(optimize(*g)),
            b: Box::new(optimize(*b)),
            intensity: Box::new(optimize(*intensity)),
        },
        Node::MeshInstance3D {
            mesh_id,
            transform,
            color_offset,
            pbr,
        } => Node::MeshInstance3D {
            mesh_id: Box::new(optimize(*mesh_id)),
            transform: Box::new(optimize(*transform)),
            color_offset: Box::new(optimize(*color_offset)),
            pbr: Box::new(optimize(*pbr)),
        },
        Node::FPSCamera { fov } => Node::FPSCamera {
            fov: Box::new(optimize(*fov)),
        },
        Node::MouseGrab { enabled } => Node::MouseGrab {
            enabled: Box::new(optimize(*enabled)),
        },
        Node::RaycastSimple => Node::RaycastSimple,
        Node::WeaponViewModel { mesh, tex } => Node::WeaponViewModel {
            mesh: Box::new(optimize(*mesh)),
            tex: Box::new(optimize(*tex)),
        },
        Node::CheckCollision {
            a_min,
            a_max,
            b_min,
            b_max,
        } => Node::CheckCollision {
            a_min: Box::new(optimize(*a_min)),
            a_max: Box::new(optimize(*a_max)),
            b_min: Box::new(optimize(*b_min)),
            b_max: Box::new(optimize(*b_max)),
        },
        Node::AddWorldAABB { min, max } => Node::AddWorldAABB {
            min: Box::new(optimize(*min)),
            max: Box::new(optimize(*max)),
        },
        Node::LoadComputeShader(val) => Node::LoadComputeShader(Box::new(optimize(*val))),
        Node::Modulo(l, r) => Node::Modulo(Box::new(optimize(*l)), Box::new(optimize(*r))),
        Node::Neg(n) => Node::Neg(Box::new(optimize(*n))),
        Node::DispatchCompute {
            shader_id,
            x,
            y,
            z,
            inputs,
        } => Node::DispatchCompute {
            shader_id: Box::new(optimize(*shader_id)),
            x: Box::new(optimize(*x)),
            y: Box::new(optimize(*y)),
            z: Box::new(optimize(*z)),
            inputs: inputs.into_iter().map(optimize).collect(),
        },
    }
}

fn optimize_math_op(left: Node, right: Node, op: char) -> Node {
    let opt_l = optimize(left);
    let opt_r = optimize(right);

    match (&opt_l, &opt_r) {
        (Node::IntLiteral(l), Node::IntLiteral(r)) => match op {
            '+' => Node::IntLiteral(l + r),
            '-' => Node::IntLiteral(l - r),
            '*' => Node::IntLiteral(l * r),
            '/' => {
                if *r != 0 {
                    Node::IntLiteral(l / r)
                } else {
                    Node::Div(Box::new(opt_l), Box::new(opt_r))
                }
            }
            _ => unreachable!(),
        },
        (Node::FloatLiteral(l), Node::FloatLiteral(r)) => match op {
            '+' => Node::FloatLiteral(l + r),
            '-' => Node::FloatLiteral(l - r),
            '*' => Node::FloatLiteral(l * r),
            '/' => {
                if *r != 0.0 {
                    Node::FloatLiteral(l / r)
                } else {
                    Node::Div(Box::new(opt_l), Box::new(opt_r))
                }
            }
            _ => unreachable!(),
        },
        _ => match op {
            '+' => Node::Add(Box::new(opt_l), Box::new(opt_r)),
            '-' => Node::Sub(Box::new(opt_l), Box::new(opt_r)),
            '*' => Node::Mul(Box::new(opt_l), Box::new(opt_r)),
            '/' => Node::Div(Box::new(opt_l), Box::new(opt_r)),
            _ => unreachable!(),
        },
    }
}

fn optimize_eq(left: Node, right: Node) -> Node {
    let opt_l = optimize(left);
    let opt_r = optimize(right);
    match (&opt_l, &opt_r) {
        (Node::IntLiteral(l), Node::IntLiteral(r)) => Node::BoolLiteral(l == r),
        (Node::FloatLiteral(l), Node::FloatLiteral(r)) => Node::BoolLiteral(l == r),
        (Node::BoolLiteral(l), Node::BoolLiteral(r)) => Node::BoolLiteral(l == r),
        (Node::StringLiteral(l), Node::StringLiteral(r)) => Node::BoolLiteral(l == r),
        _ => Node::Eq(Box::new(opt_l), Box::new(opt_r)),
    }
}

fn optimize_lt(left: Node, right: Node) -> Node {
    let opt_l = optimize(left);
    let opt_r = optimize(right);
    match (&opt_l, &opt_r) {
        (Node::IntLiteral(l), Node::IntLiteral(r)) => Node::BoolLiteral(l < r),
        (Node::FloatLiteral(l), Node::FloatLiteral(r)) => Node::BoolLiteral(l < r),
        _ => Node::Lt(Box::new(opt_l), Box::new(opt_r)),
    }
}

fn optimize_gt(left: Node, right: Node) -> Node {
    let opt_l = optimize(left);
    let opt_r = optimize(right);
    match (&opt_l, &opt_r) {
        (Node::IntLiteral(l), Node::IntLiteral(r)) => Node::BoolLiteral(l > r),
        (Node::FloatLiteral(l), Node::FloatLiteral(r)) => Node::BoolLiteral(l > r),
        _ => Node::Gt(Box::new(opt_l), Box::new(opt_r)),
    }
}

fn optimize_bitwise(left: Node, right: Node, op: char) -> Node {
    let opt_l = optimize(left);
    let opt_r = optimize(right);
    match (&opt_l, &opt_r) {
        (Node::IntLiteral(l), Node::IntLiteral(r)) => match op {
            '&' => Node::IntLiteral(l & r),
            '<' => Node::IntLiteral(l << r),
            '>' => Node::IntLiteral(l >> r),
            _ => unreachable!(),
        },
        _ => match op {
            '&' => Node::BitAnd(Box::new(opt_l), Box::new(opt_r)),
            '<' => Node::BitShiftLeft(Box::new(opt_l), Box::new(opt_r)),
            '>' => Node::BitShiftRight(Box::new(opt_l), Box::new(opt_r)),
            _ => unreachable!(),
        },
    }
}

// ── Sprint 194: Function Inlining ───────────────────────────────────
/// Attempt to inline a native FFI call at compile time if all args are literals.
/// Returns `Some(folded_node)` on success, or `None` to fall through to runtime.
fn try_inline_native(name: &str, args: &[Node]) -> Option<Node> {
    match name {
        "math_vector_scale" => {
            if args.len() == 2 {
                let (arr, factor) = (&args[0], &args[1]);
                if let (Node::ArrayCreate(elems), Node::FloatLiteral(f)) = (arr, factor) {
                    let scaled: Vec<Node> = elems
                        .iter()
                        .map(|e| match e {
                            Node::FloatLiteral(v) => Some(Node::FloatLiteral(v * f)),
                            Node::IntLiteral(v) => Some(Node::FloatLiteral(*v as f64 * f)),
                            _ => None,
                        })
                        .collect::<Option<Vec<_>>>()?;
                    return Some(Node::ArrayCreate(scaled));
                }
            }
            None
        }
        "math_sin" => inline_unary_float(args, f64::sin),
        "math_cos" => inline_unary_float(args, f64::cos),
        "math_sqrt" => inline_unary_float(args, f64::sqrt),
        "math_abs" => inline_unary_float(args, f64::abs),
        "math_tan" => inline_unary_float(args, f64::tan),
        "math_pi" => {
            if args.is_empty() {
                Some(Node::FloatLiteral(std::f64::consts::PI))
            } else {
                None
            }
        }
        "math_random" => None, // never inline non-deterministic
        "string_len" => {
            if args.len() == 1
                && let Node::StringLiteral(s) = &args[0]
            {
                Some(Node::IntLiteral(s.chars().count() as i64))
            } else {
                None
            }
        }
        "string_concat" => {
            if args.len() == 2
                && let (Node::StringLiteral(a), Node::StringLiteral(b)) = (&args[0], &args[1])
            {
                Some(Node::StringLiteral(format!("{}{}", a, b)))
            } else {
                None
            }
        }
        "string_to_upper" => {
            if args.len() == 1
                && let Node::StringLiteral(s) = &args[0]
            {
                Some(Node::StringLiteral(s.to_uppercase()))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Helper: inline a unary float function if the arg is a literal.
fn inline_unary_float(args: &[Node], f: fn(f64) -> f64) -> Option<Node> {
    if args.len() == 1
        && let Node::FloatLiteral(v) = &args[0]
    {
        Some(Node::FloatLiteral(f(*v)))
    } else {
        None
    }
}

// ---------------------------------------------------------
// TYPE INFERENCE ENGINE (SPRINT 26)
// ---------------------------------------------------------
use crate::ast::Type;
use std::collections::HashMap;

pub struct TypeChecker {
    pub scopes: Vec<HashMap<String, Type>>,
    pub errors: Vec<String>,
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            errors: Vec::new(),
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn set_var(&mut self, name: &str, t: Type) {
        // If it exists in any scope, check if the type matches. But we need to find where it is.
        for scope in self.scopes.iter_mut().rev() {
            if let Some(existing_type) = scope.get(name) {
                if *existing_type != t && *existing_type != Type::Any && t != Type::Any {
                    self.errors.push(format!(
                        "TypeError: Variable '{}' was previously assigned as {:?} but is now being assigned {:?}",
                        name, existing_type, t
                    ));
                }
                return; // Updated or conflicted
            }
        }
        // Is a new variable
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), t);
        }
    }

    pub fn get_var(&self, name: &str) -> Option<Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(t) = scope.get(name) {
                return Some(t.clone());
            }
        }
        None
    }

    pub fn check(&mut self, node: &Node) -> Result<Type, String> {
        match node {
            Node::IntLiteral(_) => Ok(Type::Int),
            Node::FloatLiteral(_) => Ok(Type::Float),
            Node::BoolLiteral(_) => Ok(Type::Bool),
            Node::StringLiteral(_) => Ok(Type::String),
            Node::ObjectLiteral(_) => Ok(Type::Object),
            Node::ArrayCreate(_) => Ok(Type::Array(vec![])),
            Node::MapCreate => Ok(Type::Map(Box::new(Type::Any))),
            Node::Identifier(name) => {
                if let Some(t) = self.get_var(name) {
                    Ok(t)
                } else {
                    Ok(Type::Any) // Unknown variables shouldn't aggressively fail if dynamically placed, or fail. Wait, let's treat as Any
                }
            }
            Node::Time | Node::GetLastKeypress => Ok(Type::Float),

            Node::Assign(name, val_node) => {
                let expr_type = self.check(val_node)?;
                self.set_var(name, expr_type);
                Ok(Type::Void) // Assign doesn't traditionally return type in strict checks
            }

            Node::Add(l, r) | Node::Sub(l, r) | Node::Mul(l, r) | Node::Div(l, r) => {
                let lt = self.check(l)?;
                let rt = self.check(r)?;
                if lt == Type::Handle || rt == Type::Handle {
                    self.errors.push(
                        "TypeError: Cannot perform mathematics on Handle pointers".to_string(),
                    );
                }
                if lt != rt && lt != Type::Any && rt != Type::Any {
                    self.errors
                        .push(format!("TypeError: Math mismatch {:?} and {:?}", lt, rt));
                }
                Ok(lt) // Assume left type dominant for now
            }
            Node::Eq(l, r) | Node::Lt(l, r) | Node::Gt(l, r) => {
                let _lt = self.check(l)?;
                let _rt = self.check(r)?;
                Ok(Type::Bool)
            }
            Node::If(cond, then_b, else_b) => {
                let ct = self.check(cond)?;
                if ct != Type::Bool && ct != Type::Any {
                    self.errors.push(format!(
                        "TypeError: 'If' condition expects Bool, found {:?}",
                        ct
                    ));
                }
                self.push_scope();
                self.check(then_b)?;
                self.pop_scope();

                if let Some(eb) = else_b {
                    self.push_scope();
                    self.check(eb)?;
                    self.pop_scope();
                }
                Ok(Type::Void)
            }
            Node::While(cond, body) => {
                let ct = self.check(cond)?;
                if ct != Type::Bool && ct != Type::Any {
                    self.errors.push(format!(
                        "TypeError: 'While' condition expects Bool, found {:?}",
                        ct
                    ));
                }
                self.push_scope();
                self.check(body)?;
                self.pop_scope();
                Ok(Type::Void)
            }
            Node::Block(nodes) => {
                self.push_scope();
                for n in nodes {
                    self.check(n)?;
                }
                self.pop_scope();
                Ok(Type::Void)
            }

            // FFI Extern Call
            Node::ExternCall {
                module: _module,
                function: _function,
                args,
            } => {
                // To safely implement this, we normally look up a signature.
                // For Sprint 26 rules: Argument types must match what NativeModule says.
                // We'll trust run_aec.rs to bind signatures, or for now, we just traverse args to mark them.
                for arg in args {
                    self.check(arg)?;
                }
                Ok(Type::Any)
            }

            // ToString always produces a String
            Node::ToString(inner) => {
                self.check(inner)?;
                Ok(Type::String)
            }

            _ => {
                // Fallback catch-all for node types we haven't strictly typed yet
                // The optimizer shouldn't block Graphics or Arrays without specific rules
                Ok(Type::Any)
            }
        }
    }
}

// ── Sprint 192: Constant Folding & Dead Code Elimination Tests ──────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Node;

    fn lit_i(v: i64) -> Node {
        Node::IntLiteral(v)
    }
    fn lit_f(v: f64) -> Node {
        Node::FloatLiteral(v)
    }
    fn lit_b(v: bool) -> Node {
        Node::BoolLiteral(v)
    }
    fn add(l: Node, r: Node) -> Node {
        Node::Add(Box::new(l), Box::new(r))
    }
    fn sub(l: Node, r: Node) -> Node {
        Node::Sub(Box::new(l), Box::new(r))
    }
    fn mul(l: Node, r: Node) -> Node {
        Node::Mul(Box::new(l), Box::new(r))
    }
    fn div(l: Node, r: Node) -> Node {
        Node::Div(Box::new(l), Box::new(r))
    }
    fn eq(l: Node, r: Node) -> Node {
        Node::Eq(Box::new(l), Box::new(r))
    }
    fn lt(l: Node, r: Node) -> Node {
        Node::Lt(Box::new(l), Box::new(r))
    }
    fn gt(l: Node, r: Node) -> Node {
        Node::Gt(Box::new(l), Box::new(r))
    }

    // ── Constant Folding: Math Ops ──────────────────────────────────

    #[test]
    fn fold_add_int() {
        assert_eq!(optimize(add(lit_i(5), lit_i(10))), lit_i(15));
    }

    #[test]
    fn fold_sub_int() {
        assert_eq!(optimize(sub(lit_i(100), lit_i(37))), lit_i(63));
    }

    #[test]
    fn fold_mul_int() {
        assert_eq!(optimize(mul(lit_i(6), lit_i(7))), lit_i(42));
    }

    #[test]
    fn fold_div_int() {
        assert_eq!(optimize(div(lit_i(60), lit_i(5))), lit_i(12));
    }

    #[test]
    fn fold_add_float() {
        assert_eq!(optimize(add(lit_f(2.5), lit_f(3.5))), lit_f(6.0));
    }

    #[test]
    fn fold_mul_float() {
        assert_eq!(optimize(mul(lit_f(3.0), lit_f(4.0))), lit_f(12.0));
    }

    #[test]
    fn fold_div_float() {
        assert_eq!(optimize(div(lit_f(10.0), lit_f(4.0))), lit_f(2.5));
    }

    /// 5 + 10 * 2 → operator precedence: Mul first → IntLiteral(25)
    #[test]
    fn fold_nested_expression() {
        // 5 + 10 * 2  →  (5 + (10 * 2)) parsed as add(5, mul(10, 2))
        let expr = add(lit_i(5), mul(lit_i(10), lit_i(2)));
        assert_eq!(optimize(expr), lit_i(25));
    }

    /// (1 + 2) * 3 → IntLiteral(9)
    #[test]
    fn fold_parenthesized_expression() {
        let expr = mul(add(lit_i(1), lit_i(2)), lit_i(3));
        assert_eq!(optimize(expr), lit_i(9));
    }

    /// Deeply nested: ((4 + 6) * 2) / 5 → IntLiteral(4)
    #[test]
    fn fold_deeply_nested() {
        let expr = div(mul(add(lit_i(4), lit_i(6)), lit_i(2)), lit_i(5));
        assert_eq!(optimize(expr), lit_i(4));
    }

    /// Division by zero must NOT be folded — keep the Div node intact
    #[test]
    fn no_fold_div_by_zero_int() {
        let expr = div(lit_i(10), lit_i(0));
        let result = optimize(expr);
        assert!(matches!(result, Node::Div(..)));
    }

    /// Division by zero for floats must NOT be folded
    #[test]
    fn no_fold_div_by_zero_float() {
        let expr = div(lit_f(10.0), lit_f(0.0));
        let result = optimize(expr);
        assert!(matches!(result, Node::Div(..)));
    }

    // ── Constant Folding: Logical Comparisons ───────────────────────

    #[test]
    fn fold_eq_int_true() {
        assert_eq!(optimize(eq(lit_i(42), lit_i(42))), lit_b(true));
    }

    #[test]
    fn fold_eq_int_false() {
        assert_eq!(optimize(eq(lit_i(42), lit_i(99))), lit_b(false));
    }

    #[test]
    fn fold_eq_float() {
        assert_eq!(optimize(eq(lit_f(1.0), lit_f(1.0))), lit_b(true));
    }

    #[test]
    fn fold_lt_int() {
        assert_eq!(optimize(lt(lit_i(3), lit_i(7))), lit_b(true));
        assert_eq!(optimize(lt(lit_i(7), lit_i(3))), lit_b(false));
    }

    #[test]
    fn fold_gt_int() {
        assert_eq!(optimize(gt(lit_i(100), lit_i(50))), lit_b(true));
        assert_eq!(optimize(gt(lit_i(10), lit_i(10))), lit_b(false));
    }

    /// Folding a comparison inside a math expression: (if (5 == 5) { 10 } else { 0 }) * 2 → 20
    #[test]
    fn fold_comparison_in_expression() {
        let if_node = Node::If(
            Box::new(eq(lit_i(5), lit_i(5))),
            Box::new(lit_i(10)),
            Some(Box::new(lit_i(0))),
        );
        let expr = mul(if_node, lit_i(2));
        // If folds to 10 (true branch), then 10 * 2 = 20
        assert_eq!(optimize(expr), lit_i(20));
    }

    // ── Dead Code Elimination: If Nodes ────────────────────────────

    #[test]
    fn dce_if_true_eliminates_false_branch() {
        let if_node = Node::If(
            Box::new(lit_b(true)),
            Box::new(lit_i(42)),
            Some(Box::new(lit_i(0))),
        );
        assert_eq!(optimize(if_node), lit_i(42));
    }

    #[test]
    fn dce_if_false_with_else_uses_else() {
        let if_node = Node::If(
            Box::new(lit_b(false)),
            Box::new(lit_i(42)),
            Some(Box::new(lit_i(99))),
        );
        assert_eq!(optimize(if_node), lit_i(99));
    }

    #[test]
    fn dce_if_false_no_else_removes_node() {
        let if_node = Node::If(Box::new(lit_b(false)), Box::new(lit_i(42)), None);
        assert_eq!(optimize(if_node), Node::Block(vec![]));
    }

    /// If condition is a folded expression that resolves to true
    #[test]
    fn dce_if_folded_condition_true() {
        // if (10 > 5) { 100 } else { 0 }  → 10 > 5 folds to true → then-branch
        let if_node = Node::If(
            Box::new(gt(lit_i(10), lit_i(5))),
            Box::new(lit_i(100)),
            Some(Box::new(lit_i(0))),
        );
        assert_eq!(optimize(if_node), lit_i(100));
    }

    /// Nested: if (if (true) { true } else { false }) { 1 } else { 2 } → 1
    #[test]
    fn dce_nested_if() {
        let inner_if = Node::If(
            Box::new(lit_b(true)),
            Box::new(lit_b(true)),
            Some(Box::new(lit_b(false))),
        );
        let outer_if = Node::If(
            Box::new(inner_if),
            Box::new(lit_i(1)),
            Some(Box::new(lit_i(2))),
        );
        assert_eq!(optimize(outer_if), lit_i(1));
    }

    // ── While Loop Dead Code Elimination ───────────────────────────

    #[test]
    fn dce_while_false_removes_loop() {
        let while_node = Node::While(Box::new(lit_b(false)), Box::new(lit_i(999)));
        assert_eq!(optimize(while_node), Node::Block(vec![]));
    }

    // ── Mixed: Constant Folding + Dead Code Elimination ────────────

    /// if (5 + 5 == 10) { 50 / 5 } else { 0 * 99 } → IntLiteral(10)
    #[test]
    fn full_pipeline_math_cmp_dce() {
        let cond = eq(add(lit_i(5), lit_i(5)), lit_i(10));
        let then_branch = div(lit_i(50), lit_i(5));
        let else_branch = mul(lit_i(0), lit_i(99));
        let if_node = Node::If(
            Box::new(cond),
            Box::new(then_branch),
            Some(Box::new(else_branch)),
        );
        assert_eq!(optimize(if_node), lit_i(10));
    }

    /// Verify fold preserves node count: 7 nodes → 1 node
    #[test]
    fn fold_reduces_node_count() {
        let expr = add(add(lit_i(1), lit_i(2)), add(lit_i(3), lit_i(4)));
        let before = count_nodes(&expr);
        assert_eq!(before, 7); // 4 literals + 3 Add nodes
        let after = count_nodes(&optimize(expr));
        assert_eq!(after, 1); // single IntLiteral(10)
    }

    /// Subtract zero should fold: x - 0 → x (identity)
    #[test]
    fn fold_sub_zero_int() {
        assert_eq!(optimize(sub(lit_i(42), lit_i(0))), lit_i(42));
    }

    /// Multiply by one should fold: x * 1 → x (identity)
    #[test]
    fn fold_mul_one_int() {
        assert_eq!(optimize(mul(lit_i(42), lit_i(1))), lit_i(42));
    }

    // ── Bitwise Constant Folding ───────────────────────────────────

    #[test]
    fn fold_bitwise_and() {
        let expr = Node::BitAnd(Box::new(lit_i(0xFF)), Box::new(lit_i(0x0F)));
        assert_eq!(optimize(expr), lit_i(0x0F));
    }

    // ── Sprint 194: Function Inlining Tests ────────────────────────

    fn native(name: &str, args: Vec<Node>) -> Node {
        Node::NativeCall(name.to_string(), args)
    }
    fn arr(elems: Vec<Node>) -> Node {
        Node::ArrayCreate(elems)
    }

    #[test]
    fn inline_math_vector_scale_float() {
        // math_vector_scale([1.0, 2.0], 2.0) → [2.0, 4.0]
        let call = native(
            "math_vector_scale",
            vec![arr(vec![lit_f(1.0), lit_f(2.0), lit_f(3.0)]), lit_f(2.0)],
        );
        assert_eq!(
            optimize(call),
            arr(vec![lit_f(2.0), lit_f(4.0), lit_f(6.0)])
        );
    }

    #[test]
    fn inline_math_sin() {
        let call = native("math_sin", vec![lit_f(std::f64::consts::PI / 2.0)]);
        assert_eq!(optimize(call), lit_f(1.0));
    }

    #[test]
    fn inline_math_cos() {
        let call = native("math_cos", vec![lit_f(0.0)]);
        assert_eq!(optimize(call), lit_f(1.0));
    }

    #[test]
    fn inline_math_sqrt() {
        let call = native("math_sqrt", vec![lit_f(16.0)]);
        assert_eq!(optimize(call), lit_f(4.0));
    }

    #[test]
    fn inline_math_abs() {
        let call = native("math_abs", vec![lit_f(-42.0)]);
        assert_eq!(optimize(call), lit_f(42.0));
    }

    #[test]
    fn inline_math_pi() {
        let call = native("math_pi", vec![]);
        assert_eq!(optimize(call), lit_f(std::f64::consts::PI));
    }

    #[test]
    fn no_inline_math_random() {
        // Random must NOT be inlined
        let call = native("math_random", vec![lit_f(0.0), lit_f(1.0)]);
        let result = optimize(call);
        assert!(matches!(result, Node::NativeCall(..)));
    }

    #[test]
    fn inline_string_len() {
        let call = native("string_len", vec![Node::StringLiteral("hello".into())]);
        assert_eq!(optimize(call), lit_i(5));
    }

    #[test]
    fn inline_string_concat() {
        let call = native(
            "string_concat",
            vec![
                Node::StringLiteral("Hello ".into()),
                Node::StringLiteral("World".into()),
            ],
        );
        assert_eq!(optimize(call), Node::StringLiteral("Hello World".into()));
    }

    #[test]
    fn inline_string_to_upper() {
        let call = native("string_to_upper", vec![Node::StringLiteral("hello".into())]);
        assert_eq!(optimize(call), Node::StringLiteral("HELLO".into()));
    }

    /// Chain: math_vector_scale + array indexing + if + DCE → single literal
    /// if (math_vector_scale([2.0], 2.0)[0] == 4.0) { 100 } else { 0 } → IntLiteral(100)
    #[test]
    fn inline_fold_dce_chain() {
        // math_vector_scale([2.0], 2.0) → [4.0]
        let scaled = native("math_vector_scale", vec![arr(vec![lit_f(2.0)]), lit_f(2.0)]);
        // [4.0][0] → 4.0
        let indexed = Node::ArrayGet(Box::new(scaled), Box::new(lit_i(0)));
        // 4.0 == 4.0 → true
        let cond = eq(indexed, lit_f(4.0));
        // if (true) { 100 } else { 0 } → 100
        let if_node = Node::If(
            Box::new(cond),
            Box::new(lit_i(100)),
            Some(Box::new(lit_i(0))),
        );
        assert_eq!(optimize(if_node), lit_i(100));
    }

    /// Chain: string_len + comparison + if → single literal
    /// if (string_len("abc") == 3) { 1 } else { 0 } → IntLiteral(1)
    #[test]
    fn inline_string_chain_to_literal() {
        let len = native("string_len", vec![Node::StringLiteral("abc".into())]);
        let cond = eq(len, lit_i(3));
        let if_node = Node::If(Box::new(cond), Box::new(lit_i(1)), Some(Box::new(lit_i(0))));
        assert_eq!(optimize(if_node), lit_i(1));
    }
}
