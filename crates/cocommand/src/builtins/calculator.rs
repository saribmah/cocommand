//! Calculator built-in app: eval and parse tools.

use serde_json::json;

use crate::routing::RoutingMetadata;
use crate::tools::schema::{RiskLevel, ToolDefinition};
use crate::tools::registry::ToolRegistry;
use crate::routing::Router;

/// App identifier.
pub const APP_ID: &str = "calculator";

/// Register calculator tools and routing metadata.
pub fn register(registry: &mut ToolRegistry, router: &mut Router) {
    registry.register_kernel_tool(eval_tool());
    registry.register_kernel_tool(parse_tool());
    router.register(routing_metadata());
}

/// Routing metadata for the calculator app.
fn routing_metadata() -> RoutingMetadata {
    RoutingMetadata {
        app_id: APP_ID.to_string(),
        keywords: vec![
            "calculate".into(),
            "math".into(),
            "compute".into(),
            "eval".into(),
        ],
        examples: vec![
            "calculate 2+2".into(),
            "what is 15% of 200".into(),
            "evaluate 3 * (4 + 5)".into(),
        ],
        verbs: vec![
            "calculate".into(),
            "compute".into(),
            "eval".into(),
            "solve".into(),
        ],
        objects: vec![
            "expression".into(),
            "math".into(),
            "calculation".into(),
        ],
    }
}

/// Tool definition for `calculator.eval`.
fn eval_tool() -> ToolDefinition {
    ToolDefinition {
        tool_id: "calculator.eval".to_string(),
        input_schema: json!({
            "type": "object",
            "required": ["expression"],
            "properties": {
                "expression": {"type": "string"}
            }
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "result": {"type": "number"},
                "expression": {"type": "string"}
            }
        }),
        risk_level: RiskLevel::Safe,
        is_kernel: false,
        handler: Box::new(|args, _ctx| {
            let expr = args
                .get("expression")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let result = evaluate(expr)?;
            Ok(json!({
                "result": result,
                "expression": expr
            }))
        }),
    }
}

/// Tool definition for `calculator.parse`.
fn parse_tool() -> ToolDefinition {
    ToolDefinition {
        tool_id: "calculator.parse".to_string(),
        input_schema: json!({
            "type": "object",
            "required": ["expression"],
            "properties": {
                "expression": {"type": "string"}
            }
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "valid": {"type": "boolean"},
                "tokens": {"type": "array"}
            }
        }),
        risk_level: RiskLevel::Safe,
        is_kernel: false,
        handler: Box::new(|args, _ctx| {
            let expr = args
                .get("expression")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let tokens = tokenize(expr);
            let valid = evaluate(expr).is_ok();
            Ok(json!({
                "valid": valid,
                "tokens": tokens,
                "expression": expr
            }))
        }),
    }
}

// --- Minimal arithmetic evaluator ---

/// Token types for the expression parser.
#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f64),
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    LParen,
    RParen,
}

/// Tokenize an arithmetic expression string.
fn tokenize(input: &str) -> Vec<serde_json::Value> {
    let tokens = lex(input);
    tokens
        .iter()
        .map(|t| match t {
            Token::Number(n) => json!({"type": "number", "value": n}),
            Token::Plus => json!({"type": "operator", "value": "+"}),
            Token::Minus => json!({"type": "operator", "value": "-"}),
            Token::Star => json!({"type": "operator", "value": "*"}),
            Token::Slash => json!({"type": "operator", "value": "/"}),
            Token::Percent => json!({"type": "operator", "value": "%"}),
            Token::LParen => json!({"type": "paren", "value": "("}),
            Token::RParen => json!({"type": "paren", "value": ")"}),
        })
        .collect()
}

/// Lexer: convert expression string into tokens.
fn lex(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            ' ' | '\t' | '\n' | '\r' => {
                chars.next();
            }
            '0'..='9' | '.' => {
                let mut num_str = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_digit() || c == '.' {
                        num_str.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if let Ok(n) = num_str.parse::<f64>() {
                    tokens.push(Token::Number(n));
                }
            }
            '+' => { tokens.push(Token::Plus); chars.next(); }
            '-' => { tokens.push(Token::Minus); chars.next(); }
            '*' => { tokens.push(Token::Star); chars.next(); }
            '/' => { tokens.push(Token::Slash); chars.next(); }
            '%' => { tokens.push(Token::Percent); chars.next(); }
            '(' => { tokens.push(Token::LParen); chars.next(); }
            ')' => { tokens.push(Token::RParen); chars.next(); }
            _ => { chars.next(); } // skip unknown chars
        }
    }

    tokens
}

/// Evaluate an arithmetic expression string.
pub fn evaluate(input: &str) -> crate::error::CoreResult<f64> {
    let tokens = lex(input);
    if tokens.is_empty() {
        return Err(crate::error::CoreError::InvalidInput(
            "empty expression".to_string(),
        ));
    }
    let mut pos = 0;
    let result = parse_expr(&tokens, &mut pos)?;
    if pos != tokens.len() {
        return Err(crate::error::CoreError::InvalidInput(
            "unexpected tokens after expression".to_string(),
        ));
    }
    Ok(result)
}

/// Parse addition/subtraction level.
fn parse_expr(tokens: &[Token], pos: &mut usize) -> crate::error::CoreResult<f64> {
    let mut left = parse_term(tokens, pos)?;
    while *pos < tokens.len() {
        match &tokens[*pos] {
            Token::Plus => {
                *pos += 1;
                left += parse_term(tokens, pos)?;
            }
            Token::Minus => {
                *pos += 1;
                left -= parse_term(tokens, pos)?;
            }
            _ => break,
        }
    }
    Ok(left)
}

/// Parse multiplication/division/modulo level.
fn parse_term(tokens: &[Token], pos: &mut usize) -> crate::error::CoreResult<f64> {
    let mut left = parse_unary(tokens, pos)?;
    while *pos < tokens.len() {
        match &tokens[*pos] {
            Token::Star => {
                *pos += 1;
                left *= parse_unary(tokens, pos)?;
            }
            Token::Slash => {
                *pos += 1;
                let right = parse_unary(tokens, pos)?;
                if right == 0.0 {
                    return Err(crate::error::CoreError::InvalidInput(
                        "division by zero".to_string(),
                    ));
                }
                left /= right;
            }
            Token::Percent => {
                *pos += 1;
                let right = parse_unary(tokens, pos)?;
                if right == 0.0 {
                    return Err(crate::error::CoreError::InvalidInput(
                        "modulo by zero".to_string(),
                    ));
                }
                left %= right;
            }
            _ => break,
        }
    }
    Ok(left)
}

/// Parse unary minus.
fn parse_unary(tokens: &[Token], pos: &mut usize) -> crate::error::CoreResult<f64> {
    if *pos < tokens.len() && tokens[*pos] == Token::Minus {
        *pos += 1;
        let val = parse_primary(tokens, pos)?;
        return Ok(-val);
    }
    parse_primary(tokens, pos)
}

/// Parse primary: number or parenthesized expression.
fn parse_primary(tokens: &[Token], pos: &mut usize) -> crate::error::CoreResult<f64> {
    if *pos >= tokens.len() {
        return Err(crate::error::CoreError::InvalidInput(
            "unexpected end of expression".to_string(),
        ));
    }

    match &tokens[*pos] {
        Token::Number(n) => {
            let val = *n;
            *pos += 1;
            Ok(val)
        }
        Token::LParen => {
            *pos += 1;
            let val = parse_expr(tokens, pos)?;
            if *pos >= tokens.len() || tokens[*pos] != Token::RParen {
                return Err(crate::error::CoreError::InvalidInput(
                    "missing closing parenthesis".to_string(),
                ));
            }
            *pos += 1;
            Ok(val)
        }
        _ => Err(crate::error::CoreError::InvalidInput(
            "unexpected token".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{MemoryStorage, Storage};
    use crate::tools::schema::ExecutionContext;
    use crate::workspace::Workspace;

    #[test]
    fn eval_basic_addition() {
        assert_eq!(evaluate("2 + 3").unwrap(), 5.0);
    }

    #[test]
    fn eval_operator_precedence() {
        assert_eq!(evaluate("2 + 3 * 4").unwrap(), 14.0);
    }

    #[test]
    fn eval_parentheses() {
        assert_eq!(evaluate("(2 + 3) * 4").unwrap(), 20.0);
    }

    #[test]
    fn eval_division() {
        assert_eq!(evaluate("10 / 2").unwrap(), 5.0);
    }

    #[test]
    fn eval_modulo() {
        assert_eq!(evaluate("10 % 3").unwrap(), 1.0);
    }

    #[test]
    fn eval_unary_minus() {
        assert_eq!(evaluate("-5 + 3").unwrap(), -2.0);
    }

    #[test]
    fn eval_nested_parens() {
        assert_eq!(evaluate("((2 + 3) * (4 - 1))").unwrap(), 15.0);
    }

    #[test]
    fn eval_division_by_zero() {
        assert!(evaluate("1 / 0").is_err());
    }

    #[test]
    fn eval_empty_expression() {
        assert!(evaluate("").is_err());
    }

    #[test]
    fn eval_decimals() {
        let result = evaluate("1.5 + 2.5").unwrap();
        assert!((result - 4.0).abs() < f64::EPSILON);
    }

    #[test]
    fn eval_tool_returns_result() {
        let tool = eval_tool();
        let mut ws = Workspace::new("test".to_string());
        let mut storage: Box<dyn Storage> = Box::new(MemoryStorage::new());
        let (event_log, clipboard_store) = storage.split_event_clipboard_mut();
        let mut ctx = ExecutionContext {
            workspace: &mut ws,
            event_log,
            clipboard_store,
        };
        let args = json!({"expression": "2 + 2"});
        let result = (tool.handler)(&args, &mut ctx).unwrap();
        assert_eq!(result["result"], 4.0);
        assert_eq!(result["expression"], "2 + 2");
    }

    #[test]
    fn parse_tool_valid_expression() {
        let tool = parse_tool();
        let mut ws = Workspace::new("test".to_string());
        let mut storage: Box<dyn Storage> = Box::new(MemoryStorage::new());
        let (event_log, clipboard_store) = storage.split_event_clipboard_mut();
        let mut ctx = ExecutionContext {
            workspace: &mut ws,
            event_log,
            clipboard_store,
        };
        let args = json!({"expression": "3 * 4"});
        let result = (tool.handler)(&args, &mut ctx).unwrap();
        assert_eq!(result["valid"], true);
        assert!(result["tokens"].as_array().unwrap().len() == 3);
    }

    #[test]
    fn parse_tool_invalid_expression() {
        let tool = parse_tool();
        let mut ws = Workspace::new("test".to_string());
        let mut storage: Box<dyn Storage> = Box::new(MemoryStorage::new());
        let (event_log, clipboard_store) = storage.split_event_clipboard_mut();
        let mut ctx = ExecutionContext {
            workspace: &mut ws,
            event_log,
            clipboard_store,
        };
        let args = json!({"expression": "3 * * 4"});
        let result = (tool.handler)(&args, &mut ctx).unwrap();
        assert_eq!(result["valid"], false);
    }

    #[test]
    fn routing_metadata_has_correct_app_id() {
        let meta = routing_metadata();
        assert_eq!(meta.app_id, "calculator");
        assert!(!meta.keywords.is_empty());
        assert!(!meta.verbs.is_empty());
    }
}
