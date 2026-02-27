use crate::ast::Node;
use std::fs;
use std::io::Error as IoError;

pub struct Parser;

impl Parser {
    /// Loads a compiled AetherCore AST from a binary file on disk.
    pub fn parse_file(path: &str) -> Result<Node, String> {
        let binary_data =
            fs::read(path).map_err(|e: IoError| format!("Failed to read file {}: {}", path, e))?;
        Self::parse_bytes(&binary_data)
    }

    /// Deserializes in-memory Bincode bytes into a structural Node.
    pub fn parse_bytes(data: &[u8]) -> Result<Node, String> {
        bincode::deserialize(data).map_err(|e| format!("Binary parser error: {}", e))
    }
}
