use knoten_core::ast::Node;
use serde_json;
use std::fs;

fn lit_int(v: i64) -> Node { Node::IntLiteral(v) }
fn lit_str(v: &str) -> Node { Node::StringLiteral(v.to_string()) }
fn lit_float(v: f64) -> Node { Node::FloatLiteral(v) }
fn get_id(name: &str) -> Node { Node::Identifier(name.to_string()) }
fn assign(name: &str, ex: Node) -> Node { Node::Assign(name.to_string(), Box::new(ex)) }

fn generate_chess_board() -> Node {
    let mut cells = Vec::new();
    for i in 0..64 {
        // Read string from array `board_state` at index `i`
        let get_piece = Node::ArrayGet(
            Box::new(get_id("board_state")),
            Box::new(lit_int(i))
        );
        
        let button = Node::UIButton(Box::new(get_piece));
        
        // Handle click
        let if_clicked = Node::If(
            Box::new(assign("clicked", button)), // Not strictly a boolean check inline, but if evaluates to true, we trigger
            Box::new(Node::Block(vec![
                Node::Print(Box::new(lit_str(&format!("Tile {} clicked", i)))),
                // We will implement move logic here in Sprint 66
            ])),
            None
        );
        cells.push(if_clicked);
    }
    
    Node::UIGrid(
        8,
        "chess_board_grid".to_string(),
        Box::new(Node::Block(cells))
    )
}

fn create_initial_board() -> Vec<Node> {
    let initial = vec![
        "♖", "♘", "♗", "♕", "♔", "♗", "♘", "♖",
        "♙", "♙", "♙", "♙", "♙", "♙", "♙", "♙",
        " ", " ", " ", " ", " ", " ", " ", " ",
        " ", " ", " ", " ", " ", " ", " ", " ",
        " ", " ", " ", " ", " ", " ", " ", " ",
        " ", " ", " ", " ", " ", " ", " ", " ",
        "♟", "♟", "♟", "♟", "♟", "♟", "♟", "♟",
        "♜", "♞", "♝", "♛", "♚", "♝", "♞", "♜",
    ];
    initial.into_iter().map(lit_str).collect()
}

fn main() {
    // 1. Initialize Board
    let setup_board = assign("board_state", Node::ArrayCreate(create_initial_board()));
    
    // 2. UISetStyle for Premium Look (Dark mode / Cyber highlighting)
    let style_setup = Node::UISetStyle(
        Box::new(lit_float(8.0)), // Rounding
        Box::new(lit_float(12.0)), // Spacing
        Box::new(Node::ArrayCreate(vec![
            lit_float(0.0), lit_float(0.8), lit_float(0.4), lit_float(1.0) // Accent (Cyber Green)
        ])),
        Box::new(Node::ArrayCreate(vec![
            lit_float(0.08), lit_float(0.08), lit_float(0.12), lit_float(0.95) // Dark Glass Fill
        ])),
        None, // Default Idle
        Some(Box::new(Node::ArrayCreate(vec![
            lit_float(0.2), lit_float(0.8), lit_float(0.4), lit_float(0.8) // Hover
        ])))
    );

    // 3. UI App Loop
    let main_window = Node::UIWindow(
        "chess_main".to_string(),
        Box::new(lit_str("Agentic WGPU Chess")),
        Box::new(Node::Block(vec![
            style_setup,
            Node::UILabel(Box::new(lit_str("Chess Engine vs. Player"))),
            generate_chess_board()
        ]))
    );

    let program = Node::Block(vec![
        setup_board,
        Node::While(
            Box::new(Node::BoolLiteral(true)),
            Box::new(Node::Block(vec![
                Node::InitGraphics, // Polls/Renders the WGPU EGUI frames
                main_window,
                Node::PollEvents(Box::new(Node::Block(vec![])))
            ]))
        )
    ]);

    let json = serde_json::to_string_pretty(&program).unwrap();
    let out_dir = "examples/graphics";
    fs::create_dir_all(out_dir).unwrap();
    let out_path = format!("{}/chess_showcase.nod", out_dir);
    
    fs::write(&out_path, json).unwrap();
    println!("Compiled AST to {}", out_path);
}
