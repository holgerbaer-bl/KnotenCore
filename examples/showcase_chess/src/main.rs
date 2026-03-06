use knoten_core::ast::Node;
use knoten_core::executor::ExecutionEngine;
use std::fs;
use serde_json;

fn lit_int(v: i64) -> Node { Node::IntLiteral(v) }
fn lit_str(v: &str) -> Node { Node::StringLiteral(v.to_string()) }
fn lit_float(v: f64) -> Node { Node::FloatLiteral(v) }
fn get_id(name: &str) -> Node { Node::Identifier(name.to_string()) }
fn assign(name: &str, ex: Node) -> Node { Node::Assign(name.to_string(), Box::new(ex)) }

/// Build an RGBA color style node.
fn rgba(r: f64, g: f64, b: f64, a: f64) -> Node {
    Node::ArrayCreate(vec![lit_float(r), lit_float(g), lit_float(b), lit_float(a)])
}

/// Build a UISetStyle node that sets only the button idle color (background).
fn tile_style(r: f64, g: f64, b: f64, hover_r: f64, hover_g: f64, hover_b: f64) -> Node {
    Node::UISetStyle(
        Box::new(lit_float(0.0)),                     // rounding = 0 (sharp corners for chess)
        Box::new(lit_float(1.0)),                     // spacing = 1
        Box::new(rgba(0.0, 0.0, 0.0, 1.0)),          // accent (unused)
        Box::new(rgba(0.0, 0.0, 0.0, 0.0)),          // fill (transparent, tile color drives it)
        Some(Box::new(rgba(r, g, b, 1.0))),           // button idle color
        Some(Box::new(rgba(hover_r, hover_g, hover_b, 1.0))), // button hover / selection
    )
}

/// Build the complete 8x8 chess board as 8 UIHorizontal rows of 8 UIButton tiles.
/// Each tile is a real graphical node with background color, not an ASCII string.
fn build_chess_board() -> Vec<Node> {
    // Colors: classic board
    let (dark_r, dark_g, dark_b) = (0.18, 0.47, 0.24);   // dark green
    let (light_r, light_g, light_b) = (0.93, 0.90, 0.76); // warm ivory

    // Selected tile golden highlight
    let (sel_r, sel_g, sel_b) = (0.85, 0.70, 0.10);      // gold

    let mut rows: Vec<Node> = Vec::new();

    for row in 0..8i64 {
        let mut cells: Vec<Node> = Vec::new();

        for col in 0..8i64 {
            let i = row * 8 + col; // flat index

            let is_dark = (row + col) % 2 == 1;
            let (bg_r, bg_g, bg_b) = if is_dark {
                (dark_r, dark_g, dark_b)
            } else {
                (light_r, light_g, light_b)
            };

            // Get piece symbol from board array
            let get_piece = Node::ArrayGet(
                Box::new(get_id("board_state")),
                Box::new(lit_int(i)),
            );

            // Is this tile selected?
            let is_selected = Node::Eq(
                Box::new(get_id("selected_index")),
                Box::new(lit_int(i)),
            );

            // Tile button text: show piece symbol with layout padding for size.
            // "\n" forces egui to give the button vertical height.
            let piece_text = Node::Concat(
                Box::new(lit_str("\n")),
                Box::new(Node::Concat(
                    Box::new(get_piece.clone()),
                    Box::new(lit_str("\n")),
                )),
            );

            // Tile style: gold if selected, otherwise board color
            let style_node = Node::If(
                Box::new(is_selected.clone()),
                Box::new(tile_style(sel_r, sel_g, sel_b, 1.0, 0.85, 0.20)),
                Some(Box::new(tile_style(bg_r, bg_g, bg_b, bg_r + 0.1, bg_g + 0.05, bg_b + 0.05))),
            );

            // The actual button — styled tile
            let button = Node::UIButton(Box::new(piece_text));

            // Click logic
            let click_logic = Node::If(
                Box::new(Node::Eq(Box::new(get_id("selected_index")), Box::new(lit_int(-1)))),
                // Nothing selected: select this cell (only if not empty)
                Box::new(Node::If(
                    Box::new(Node::Eq(Box::new(get_piece.clone()), Box::new(lit_str(" ")))),
                    Box::new(Node::Block(vec![])), // Empty tile — ignore
                    Some(Box::new(assign("selected_index", lit_int(i)))),
                )),
                // Piece already selected: move it here
                Some(Box::new(Node::Block(vec![
                    Node::ArraySet(
                        Box::new(get_id("board_state")),
                        Box::new(lit_int(i)),
                        Box::new(Node::ArrayGet(
                            Box::new(get_id("board_state")),
                            Box::new(get_id("selected_index")),
                        )),
                    ),
                    Node::ArraySet(
                        Box::new(get_id("board_state")),
                        Box::new(get_id("selected_index")),
                        Box::new(lit_str(" ")),
                    ),
                    assign("selected_index", lit_int(-1)),
                    // Switch turn after move
                    assign("turn", Node::If(
                        Box::new(Node::Eq(Box::new(get_id("turn")), Box::new(lit_int(0)))),
                        Box::new(lit_int(1)),
                        Some(Box::new(lit_int(0))),
                    )),
                    Node::Store { key: "chess_board".to_string(), value: Box::new(get_id("board_state")) },
                    Node::Store { key: "chess_turn".to_string(), value: Box::new(get_id("turn")) },
                ]))),
            );

            // One cell = [style, if(button_clicked) -> click_logic]
            cells.push(Node::Block(vec![
                style_node,
                Node::If(Box::new(button), Box::new(click_logic), None),
            ]));
        }

        // One row = UIHorizontal wrapping 8 cells
        rows.push(Node::UIHorizontal(Box::new(Node::Block(cells))));
    }

    rows
}

fn run() {
    println!("Loading chess AST...");
    let nod_path = "../../examples/graphics/chess_showcase.nod";
    let json_string = fs::read_to_string(nod_path)
        .expect("Failed to read chess_showcase.nod — run chess_builder first!");
    let mut ast: Node = serde_json::from_str(&json_string)
        .expect("Failed to parse chess_showcase.nod");

    // Inject the native graphical board into the AST (replace the placeholder UIGrid)
    inject_native_board(&mut ast);

    println!("Starting KnotenCore Engine...");
    let mut engine = ExecutionEngine::new();
    engine.permissions.allow_fs_read = true;
    engine.permissions.allow_fs_write = true;

    let result = engine.execute(&ast);
    if result.starts_with("Fault:") {
        eprintln!("❌ FAULT: {}", result);
        std::process::exit(1);
    } else {
        println!("✅ Exited: {}", result);
    }
}

/// Find the UIGrid placeholder in the AST and replace it with 8 UIHorizontal rows.
fn inject_native_board(ast: &mut Node) {
    match ast {
        Node::UIGrid(_, id, _) if id == "chess_board_grid" => {
            println!(">> Replacing UIGrid '{}' with native 8x8 UIHorizontal rows", id);
            let board_rows = build_chess_board();
            *ast = Node::Block(board_rows);
        }
        Node::Block(nodes) => {
            for n in nodes.iter_mut() {
                inject_native_board(n);
            }
        }
        Node::PollEvents(body) => inject_native_board(body),
        Node::While(_, body) => inject_native_board(body),
        Node::If(_, then_b, else_b) => {
            inject_native_board(then_b);
            if let Some(e) = else_b { inject_native_board(e); }
        }
        Node::UIWindow(_, _, body) => inject_native_board(body),
        Node::UIHorizontal(body) => inject_native_board(body),
        Node::UIFullscreen(body) => inject_native_board(body),
        Node::UIScrollArea(_, body) => inject_native_board(body),
        _ => {}
    }
}

fn main() {
    // 32 MB stack for deep recursive AST; KnotenCore uses winit `any_thread` internally
    let builder = std::thread::Builder::new()
        .name("knoten-runtime".to_string())
        .stack_size(32 * 1024 * 1024);
    let handler = builder.spawn(run).expect("Failed to spawn runtime thread");
    handler.join().unwrap();
}
