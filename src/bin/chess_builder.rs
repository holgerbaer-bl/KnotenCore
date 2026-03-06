use knoten_core::ast::Node;
use serde_json;
use std::fs;

fn lit_int(v: i64) -> Node { Node::IntLiteral(v) }
fn lit_str(v: &str) -> Node { Node::StringLiteral(v.to_string()) }
fn lit_float(v: f64) -> Node { Node::FloatLiteral(v) }
fn get_id(name: &str) -> Node { Node::Identifier(name.to_string()) }
fn assign(name: &str, ex: Node) -> Node { Node::Assign(name.to_string(), Box::new(ex)) }

fn generate_chess_board() -> Node {
    // Leave cells EMPTY here — main.rs will inject 64 cells dynamically via AST manipulation.
    Node::UIGrid(
        8,
        "chess_board_grid".to_string(),
        Box::new(Node::Block(vec![])) // Empty — filled by Rust wrapper
    )
}

fn create_initial_board() -> Vec<Node> {
    let initial = vec![
        "♜", "♞", "♝", "♛", "♚", "♝", "♞", "♜",
        "♟", "♟", "♟", "♟", "♟", "♟", "♟", "♟",
        " ", " ", " ", " ", " ", " ", " ", " ",
        " ", " ", " ", " ", " ", " ", " ", " ",
        " ", " ", " ", " ", " ", " ", " ", " ",
        " ", " ", " ", " ", " ", " ", " ", " ",
        "♙", "♙", "♙", "♙", "♙", "♙", "♙", "♙",
        "♖", "♘", "♗", "♕", "♔", "♗", "♘", "♖",
    ];
    initial.into_iter().map(lit_str).collect()
}

fn main() {
    // 1. Initialize State — always default to Player (White = 0) first
    let mut setup_state = vec![
        assign("board_state", Node::ArrayCreate(create_initial_board())),
        assign("turn", lit_int(0)),         // 0 = White (Player), 1 = Black (AI)
        assign("selected_index", lit_int(-1)),

        // Try to load saved game state
        assign("board_load", Node::Load { key: "chess_board".to_string() }),
        assign("turn_load", Node::Load { key: "chess_turn".to_string() }),

        // Only restore if turn_load is NOT void (i.e., a real saved value exists)
        Node::If(
            Box::new(Node::Eq(Box::new(get_id("turn_load")), Box::new(lit_str("void")))),
            Box::new(Node::Block(vec![])), // No save found — keep defaults
            Some(Box::new(Node::Block(vec![
                // Restore board only if it's non-empty and non-void
                assign("board_state", Node::If(
                    Box::new(Node::Eq(Box::new(get_id("board_load")), Box::new(lit_str("")))),
                    Box::new(get_id("board_state")),
                    Some(Box::new(Node::If(
                        Box::new(Node::Eq(Box::new(get_id("board_load")), Box::new(lit_str("void")))),
                        Box::new(get_id("board_state")),
                        Some(Box::new(get_id("board_load")))
                    )))
                )),
                // Restore turn — but clamp to 0 (White) if invalid
                assign("turn", Node::If(
                    Box::new(Node::Eq(Box::new(get_id("turn_load")), Box::new(lit_int(1)))),
                    Box::new(lit_int(0)), // Hotfix: Force back to player on load to avoid AI deadlock
                    Some(Box::new(lit_int(0)))
                ))
            ])))
        )
    ];

    // 2. Premium dark theme style — tight spacing to give board room
    let global_style = Node::UISetStyle(
        Box::new(lit_float(4.0)),
        Box::new(lit_float(4.0)),
        Box::new(Node::ArrayCreate(vec![
            lit_float(0.0), lit_float(0.8), lit_float(0.4), lit_float(1.0)
        ])),
        Box::new(Node::ArrayCreate(vec![
            lit_float(0.05), lit_float(0.05), lit_float(0.08), lit_float(0.98)
        ])),
        None,
        None
    );

    // 3. AI logic — NON-BLOCKING: only fires when it's AI's turn.
    //    We make AI auto-pass (simplified). A real AI move would mutate board_state here.
    //    Crucially, it runs INSIDE PollEvents so it doesn't block the render loop.
    let ai_logic = Node::If(
        Box::new(Node::Eq(Box::new(get_id("turn")), Box::new(lit_int(1)))),
        Box::new(Node::Block(vec![
            // Simplified AI: just pass back to the player after one frame
            assign("turn", lit_int(0)),
            Node::Print(Box::new(lit_str("AI made its move."))),
            Node::Store { key: "chess_turn".to_string(), value: Box::new(get_id("turn")) }
        ])),
        None
    );

    // 4. Main UI window — board is between turn label and controls
    let main_window = Node::UIWindow(
        "chess_main".to_string(),
        Box::new(lit_str("Agentic WGPU Chess")),
        Box::new(Node::Block(vec![
            global_style.clone(),

            // Turn indicator
            Node::If(
                Box::new(Node::Eq(Box::new(get_id("turn")), Box::new(lit_int(0)))),
                Box::new(Node::UILabel(Box::new(lit_str("♟ Turn: Player (White)")))),
                Some(Box::new(Node::UILabel(Box::new(lit_str("🤖 Turn: AI (Black)")))))
            ),

            // Chess Board Grid (64 cells injected dynamically by main.rs)
            generate_chess_board(),

            // Reset button
            Node::UIHorizontal(Box::new(Node::Block(vec![
                Node::If(
                    Box::new(Node::UIButton(Box::new(lit_str("🔄 Reset Game")))),
                    Box::new(Node::Block(vec![
                        assign("board_state", Node::ArrayCreate(create_initial_board())),
                        assign("turn", lit_int(0)),
                        assign("selected_index", lit_int(-1)),
                        Node::Store { key: "chess_board".to_string(), value: Box::new(get_id("board_state")) },
                        Node::Store { key: "chess_turn".to_string(), value: Box::new(get_id("turn")) }
                    ])),
                    None
                )
            ]))),
        ]))
    );

    setup_state.push(Node::InitWindow(Box::new(lit_int(920)), Box::new(lit_int(740)), Box::new(lit_str("Agentic WGPU Chess"))));
    setup_state.push(Node::InitGraphics);
    // PollEvents drives the render loop — AI logic and UI run here every frame
    setup_state.push(Node::PollEvents(Box::new(Node::Block(vec![
        ai_logic,
        main_window,
    ]))));

    let program = Node::Block(setup_state);

    let json = serde_json::to_string_pretty(&program).unwrap();
    let out_dir = "examples/graphics";
    fs::create_dir_all(out_dir).unwrap();
    let out_path = format!("{}/chess_showcase.nod", out_dir);
    fs::write(&out_path, json).unwrap();
    println!("Compiled AST to {}", out_path);
}
