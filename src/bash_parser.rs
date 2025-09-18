use anyhow::Result;
use serde::Serialize;
use tree_sitter::{Node, Parser, TreeCursor};

#[derive(Debug, Serialize)]
pub struct AstNode {
    pub kind: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_position: (usize, usize),
    pub end_position: (usize, usize),
    pub children: Vec<AstNode>,
}

pub struct BashParser {
    parser: Parser,
}

impl BashParser {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        let lang = tree_sitter_bash::language();
        parser.set_language(&lang).expect("load bash grammar");
        Ok(Self { parser })
    }

    pub fn parse_to_tree(&mut self, source: &str) -> Option<tree_sitter::Tree> {
        self.parser.parse(source, None)
    }

pub fn parse_to_ast(&mut self, source: &str) -> Result<AstNode> {
        let tree = self
            .parse_to_tree(source)
            .ok_or_else(|| anyhow::anyhow!("无法解析输入为语法树"))?;
        let root = tree.root_node();
        Ok(build_node_recursive(root))
    }
}

fn build_node_recursive(node: Node) -> AstNode {
    let mut cursor: TreeCursor = node.walk();
    let mut children = Vec::new();
    for child in node.children(&mut cursor) {
        children.push(build_node_recursive(child));
    }
    AstNode {
        kind: node.kind().to_string(),
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
        start_position: (node.start_position().row, node.start_position().column),
        end_position: (node.end_position().row, node.end_position().column),
        children,
    }
}

/// Generate a concise, human-readable outline (one node per line; indentation denotes depth)
pub fn ast_outline(ast: &AstNode, indent: usize, out: &mut String) {
    let pad = " ".repeat(indent * 2);
    let _ = std::fmt::Write::write_fmt(
        out,
        format_args!(
            "{}{} [{}..{}] ({},{})->({},{})\n",
            pad,
            ast.kind,
            ast.start_byte,
            ast.end_byte,
            ast.start_position.0,
            ast.start_position.1,
            ast.end_position.0,
            ast.end_position.1
        ),
    );
    for ch in &ast.children {
        ast_outline(ch, indent + 1, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_pipeline() -> Result<()> {
        let mut p = BashParser::new()?;
        let ast = p.parse_to_ast("echo 1 | grep 1")?;
        assert_eq!(ast.kind, "program");
        Ok(())
    }
}
