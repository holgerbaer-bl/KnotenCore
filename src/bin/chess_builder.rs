use knoten_core::ast::Node;
use serde_json;
use std::fs;

fn lit_int(v: i64) -> Node { Node::IntLiteral(v) }
fn lit_str(v: &str) -> Node { Node::StringLiteral(v.to_string()) }
fn lit_float(v: f64) -> Node { Node::FloatLiteral(v) }
fn get_id(name: &str) -> Node { Node::Identifier(name.to_string()) }
fn assign(name: &str, ex: Node) -> Node { Node::Assign(name.to_string(), Box::new(ex)) }
fn rgba_arr(r: f64, g: f64, b: f64, a: f64) -> Node {
    Node::ArrayCreate(vec![lit_float(r), lit_float(g), lit_float(b), lit_float(a)])
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

/// Move a piece from src_idx to dst_idx on the board, then switch turn to 0 (player) and persist.
fn ai_move(src: i64, dst: i64) -> Node {
    Node::Block(vec![
        // Copy piece from src to dst
        Node::ArraySet(
            Box::new(get_id("board_state")),
            Box::new(lit_int(dst)),
            Box::new(Node::ArrayGet(Box::new(get_id("board_state")), Box::new(lit_int(src)))),
        ),
        // Clear src
        Node::ArraySet(
            Box::new(get_id("board_state")),
            Box::new(lit_int(src)),
            Box::new(lit_str(" ")),
        ),
        assign("turn", lit_int(0)),
        assign("selected_index", lit_int(-1)),
        Node::Store { key: "chess_board".to_string(), value: Box::new(get_id("board_state")) },
        Node::Store { key: "chess_turn".to_string(), value: Box::new(get_id("turn")) },
    ])
}

/// AI picks the first available black pawn that can move one step forward (src+8).
/// Falls back to a knight move if all pawns are blocked.
fn build_ai_logic() -> Node {
    // Try each of the 8 black pawns (initial positions 8–15), one at a time.
    // A pawn can move if position src+8 is empty.
    let mut chain: Option<Box<Node>> = None;

    // Build from back → front so the chain resolves in order (pos 8 first)
    for col in (0i64..8).rev() {
        let src = 8 + col;
        let dst = src + 8;

        let can_move = Node::Block(vec![
            // Can move if: board[src] is not empty AND board[src+8] is empty
            Node::If(
                Box::new(Node::Eq(Box::new(get_id("ai_moved")), Box::new(lit_int(0)))),
                Box::new(Node::If(
                    Box::new(Node::Eq(
                        Box::new(Node::ArrayGet(Box::new(get_id("board_state")), Box::new(lit_int(src)))),
                        Box::new(lit_str(" ")),
                    )),
                    Box::new(Node::Block(vec![])), // piece gone already — skip
                    Some(Box::new(Node::If(
                        Box::new(Node::Eq(
                            Box::new(Node::ArrayGet(Box::new(get_id("board_state")), Box::new(lit_int(dst)))),
                            Box::new(lit_str(" ")),
                        )),
                        Box::new(Node::Block(vec![
                            ai_move(src, dst),
                            assign("ai_moved", lit_int(1)),
                        ])),
                        None,
                    ))),
                )),
                None,
            ),
        ]);

        if let Some(prev) = chain {
            chain = Some(Box::new(Node::Block(vec![can_move, *prev])));
        } else {
            chain = Some(Box::new(can_move));
        }
    }

    // Wrap in: if turn == AI (1): try to move a pawn
    Node::If(
        Box::new(Node::Eq(Box::new(get_id("turn")), Box::new(lit_int(1)))),
        Box::new(Node::Block(vec![
            assign("ai_moved", lit_int(0)),
            *chain.unwrap_or_else(|| Box::new(Node::Block(vec![]))),
            // If nothing moved (all pawns blocked), just pass turn back
            Node::If(
                Box::new(Node::Eq(Box::new(get_id("ai_moved")), Box::new(lit_int(0)))),
                Box::new(Node::Block(vec![
                    assign("turn", lit_int(0)),
                    Node::Store { key: "chess_turn".to_string(), value: Box::new(get_id("turn")) },
                ])),
                None,
            ),
        ])),
        None,
    )
}

fn generate_chess_board() -> Node {
    Node::UIGrid(8, "chess_board_grid".to_string(), Box::new(Node::Block(vec![])))
}

fn main() {
    let mut setup_state = vec![
        // Safe defaults
        assign("board_state", Node::ArrayCreate(create_initial_board())),
        assign("turn", lit_int(0)),         // 0 = White (Human), 1 = Black (AI)
        assign("selected_index", lit_int(-1)),
        assign("ai_moved", lit_int(0)),

        // Try loading saved state
        assign("board_load", Node::Load { key: "chess_board".to_string() }),
        assign("turn_load", Node::Load { key: "chess_turn".to_string() }),
        Node::If(
            Box::new(Node::Eq(Box::new(get_id("turn_load")), Box::new(lit_str("void")))),
            Box::new(Node::Block(vec![])),
            Some(Box::new(Node::Block(vec![
                assign("board_state", Node::If(
                    Box::new(Node::Eq(Box::new(get_id("board_load")), Box::new(lit_str("")))),
                    Box::new(get_id("board_state")),
                    Some(Box::new(Node::If(
                        Box::new(Node::Eq(Box::new(get_id("board_load")), Box::new(lit_str("void")))),
                        Box::new(get_id("board_state")),
                        Some(Box::new(get_id("board_load"))),
                    )))
                )),
                assign("turn", get_id("turn_load")),
            ])))
        ),
    ];

    // ── Global neon cyberpunk style ───────────────────────────────────────────
    let global_style = Node::UISetStyle(
        Box::new(lit_float(6.0)),   // rounding
        Box::new(lit_float(6.0)),   // spacing
        Box::new(rgba_arr(0.0, 0.9, 0.7, 1.0)),   // accent: cyan-teal
        Box::new(rgba_arr(0.04, 0.04, 0.08, 0.98)), // dark bg
        None,
        None
    );

    // ── AI turn logic ─────────────────────────────────────────────────────────
    let ai_logic = build_ai_logic();

    // ── Main UI ───────────────────────────────────────────────────────────────
    let main_window = Node::UIWindow(
        "chess_main".to_string(),
        Box::new(lit_str("♟ Agentic WGPU Chess — Human vs. Computer")),
        Box::new(Node::Block(vec![
            global_style,

            // Turn status bar
            Node::If(
                Box::new(Node::Eq(Box::new(get_id("turn")), Box::new(lit_int(0)))),
                Box::new(Node::UILabel(Box::new(lit_str("◉  YOUR TURN  — White (Human)")))),
                Some(Box::new(Node::UILabel(Box::new(lit_str("◆  AI THINKING  — Black (Computer)"))))),
            ),

            // The board (64 cells injected by main.rs)
            generate_chess_board(),

            // Controls
            Node::UIHorizontal(Box::new(Node::Block(vec![
                Node::If(
                    Box::new(Node::UIButton(Box::new(lit_str("🔄 New Game")))),
                    Box::new(Node::Block(vec![
                        assign("board_state", Node::ArrayCreate(create_initial_board())),
                        assign("turn", lit_int(0)),
                        assign("selected_index", lit_int(-1)),
                        Node::Store { key: "chess_board".to_string(), value: Box::new(get_id("board_state")) },
                        Node::Store { key: "chess_turn".to_string(), value: Box::new(get_id("turn")) },
                    ])),
                    None,
                ),
            ]))),
        ]))
    );

    setup_state.push(Node::InitWindow(
        Box::new(lit_int(960)),
        Box::new(lit_int(760)),
        Box::new(lit_str("Agentic WGPU Chess")),
    ));
    setup_state.push(Node::InitGraphics);
    setup_state.push(Node::PollEvents(Box::new(Node::Block(vec![
        ai_logic,
        main_window,
    ]))));

    let program = Node::Block(setup_state);
    let json = serde_json::to_string_pretty(&program).unwrap();
    fs::create_dir_all("examples/graphics").unwrap();
    fs::write("examples/graphics/chess_showcase.nod", json).unwrap();
    println!("✅ AST compiled to examples/graphics/chess_showcase.nod");
}
