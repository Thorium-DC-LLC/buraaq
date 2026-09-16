//! Stable diagnostic IDs and human-readable catalog entries.

use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErrorCode {
    pub id: &'static str,
    pub title: &'static str,
    pub explanation: &'static str,
}

pub fn catalog() -> HashMap<&'static str, ErrorCode> {
    let entries = [
        ErrorCode {
            id: "E0001",
            title: "Lexical error",
            explanation: "The source contains characters or tokens the lexer cannot interpret.",
        },
        ErrorCode {
            id: "E0100",
            title: "Syntax error",
            explanation: "The parser could not build a valid AST from this construct.",
        },
        ErrorCode {
            id: "E0101",
            title: "Unknown name",
            explanation: "No binding, type, or module exists with this name in scope.",
        },
        ErrorCode {
            id: "E0102",
            title: "Missing expression",
            explanation: "An expression was expected but the source ended or was incomplete.",
        },
        ErrorCode {
            id: "E0201",
            title: "Type mismatch",
            explanation: "The expression's type is not compatible with the expected type.",
        },
        ErrorCode {
            id: "E0301",
            title: "Used before initialization",
            explanation: "A binding was read before it was assigned a value.",
        },
        ErrorCode {
            id: "E0302",
            title: "Use after transfer",
            explanation: "An owned value was moved into another place and then used again.",
        },
        ErrorCode {
            id: "E0310",
            title: "Invalid mutable borrow",
            explanation: "A binding not declared `mut` cannot be borrowed mutably.",
        },
        ErrorCode {
            id: "E0311",
            title: "Conflicting borrow",
            explanation: "An active borrow prevents this new borrow from being created.",
        },
        ErrorCode {
            id: "E0401",
            title: "Cyclic module dependency",
            explanation: "Two or more modules import each other, directly or indirectly.",
        },
        ErrorCode {
            id: "E0403",
            title: "Unresolved import",
            explanation: "The named module or symbol cannot be found from this crate or the sysroot.",
        },
        ErrorCode {
            id: "E0404",
            title: "Duplicate public symbol",
            explanation: "The same public name is exported from more than one module.",
        },
        ErrorCode {
            id: "E0412",
            title: "Private item",
            explanation: "A name is visible only in its defining module unless it is declared `pub`.",
        },
        ErrorCode {
            id: "E0312",
            title: "Unsafe dereference",
            explanation: "Dereferencing a raw pointer is only allowed inside an `unsafe` block.",
        },
        ErrorCode {
            id: "E0313",
            title: "Invalid dereference",
            explanation: "Only references and raw pointers can be dereferenced.",
        },
    ];
    entries.into_iter().map(|e| (e.id, e)).collect()
}

pub fn explain(code: &str) -> Option<&'static str> {
    catalog().get(code).map(|e| e.explanation)
}
