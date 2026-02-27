use aether_compiler::ast::Node;
use std::fs::File;
use std::io::Write;

fn main() {
    let program = Node::Block(vec![
        Node::InitWindow(
            Box::new(Node::IntLiteral(800)),
            Box::new(Node::IntLiteral(600)),
            Box::new(Node::StringLiteral("AetherCore Text Editor".to_string())),
        ),
        Node::InitGraphics,
        Node::LoadFont(Box::new(Node::StringLiteral(
            "assets/Roboto-Regular.ttf".to_string(),
        ))),
        // Initialize an empty document
        Node::Assign(
            "doc".to_string(),
            Box::new(Node::StringLiteral("".to_string())),
        ),
        Node::PollEvents(Box::new(Node::Block(vec![
            // Get keystrokes and append them natively
            Node::Assign("keys".to_string(), Box::new(Node::GetLastKeypress)),
            Node::Assign(
                "doc".to_string(),
                Box::new(Node::Add(
                    Box::new(Node::Identifier("doc".to_string())),
                    Box::new(Node::Identifier("keys".to_string())),
                )),
            ),
            // Draw it
            Node::DrawText(
                Box::new(Node::Identifier("doc".to_string())),
                Box::new(Node::FloatLiteral(40.0)),
                Box::new(Node::FloatLiteral(40.0)),
                Box::new(Node::FloatLiteral(32.0)),
                Box::new(Node::ArrayLiteral(vec![
                    Node::FloatLiteral(1.0),
                    Node::FloatLiteral(1.0),
                    Node::FloatLiteral(1.0),
                    Node::FloatLiteral(1.0),
                ])),
            ),
        ]))),
    ]);

    let bin = bincode::serialize(&program).unwrap();
    let mut file = File::create("text_editor.aec").unwrap();
    file.write_all(&bin).unwrap();

    println!("Demo generator success: text_editor.aec created.");
}
