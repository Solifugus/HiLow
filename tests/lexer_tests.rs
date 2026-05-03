use hilowc::lexer::{Lexer, TokenKind};

#[test]
fn test_integer_literal_decimal() {
    let input = "42";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 2); // number + EOF
    assert_eq!(tokens[0].kind, TokenKind::Integer(42));
    assert_eq!(tokens[0].lexeme, "42");
}

#[test]
fn test_integer_literal_hex() {
    let input = "0x1F";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].kind, TokenKind::Integer(31));
    assert_eq!(tokens[0].lexeme, "0x1F");
}

#[test]
fn test_integer_literal_binary() {
    let input = "0b1010";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].kind, TokenKind::Integer(10));
    assert_eq!(tokens[0].lexeme, "0b1010");
}

#[test]
fn test_integer_literal_with_underscores() {
    let input = "1_000";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].kind, TokenKind::Integer(1000));
    assert_eq!(tokens[0].lexeme, "1_000");
}

#[test]
fn test_float_literal_simple() {
    let input = "3.14";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].kind, TokenKind::Float(3.14));
    assert_eq!(tokens[0].lexeme, "3.14");
}

#[test]
fn test_float_literal_scientific_positive() {
    let input = "2.5e10";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].kind, TokenKind::Float(2.5e10));
    assert_eq!(tokens[0].lexeme, "2.5e10");
}

#[test]
fn test_float_literal_scientific_negative() {
    let input = "1.5e-3";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].kind, TokenKind::Float(1.5e-3));
    assert_eq!(tokens[0].lexeme, "1.5e-3");
}

#[test]
fn test_identifier_simple() {
    let input = "foo";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].kind, TokenKind::Identifier);
    assert_eq!(tokens[0].lexeme, "foo");
}

#[test]
fn test_identifier_underscore_prefix() {
    let input = "_bar";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].kind, TokenKind::Identifier);
    assert_eq!(tokens[0].lexeme, "_bar");
}

#[test]
fn test_identifier_camelcase() {
    let input = "camelCase";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].kind, TokenKind::Identifier);
    assert_eq!(tokens[0].lexeme, "camelCase");
}

#[test]
fn test_identifier_snake_case() {
    let input = "snake_case";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].kind, TokenKind::Identifier);
    assert_eq!(tokens[0].lexeme, "snake_case");
}

// Reserved-for-future keywords should lex as identifiers
#[test]
fn test_reserved_words_as_identifiers() {
    let reserved_words = ["class", "interface", "trait", "yield", "enum"];

    for word in reserved_words {
        let tokens = Lexer::new(word).tokens().unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].kind, TokenKind::Identifier);
        assert_eq!(tokens[0].lexeme, word);
    }
}

// Test all 41 keywords
#[test]
fn test_keyword_and() {
    let input = "and";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::And);
}

#[test]
fn test_keyword_arena() {
    let input = "arena";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Arena);
}

#[test]
fn test_keyword_async() {
    let input = "async";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Async);
}

#[test]
fn test_keyword_break() {
    let input = "break";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Break);
}

#[test]
fn test_keyword_case() {
    let input = "case";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Case);
}

#[test]
fn test_keyword_continue() {
    let input = "continue";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Continue);
}

#[test]
fn test_keyword_decreases() {
    let input = "decreases";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Decreases);
}

#[test]
fn test_keyword_default() {
    let input = "default";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Default);
}

#[test]
fn test_keyword_defer() {
    let input = "defer";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Defer);
}

#[test]
fn test_keyword_else() {
    let input = "else";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Else);
}

#[test]
fn test_keyword_ensures() {
    let input = "ensures";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Ensures);
}

#[test]
fn test_keyword_excluding() {
    let input = "excluding";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Excluding);
}

#[test]
fn test_keyword_export() {
    let input = "export";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Export);
}

#[test]
fn test_keyword_false() {
    let input = "false";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::False);
}

#[test]
fn test_keyword_for() {
    let input = "for";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::For);
}

#[test]
fn test_keyword_from() {
    let input = "from";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::From);
}

#[test]
fn test_keyword_function() {
    let input = "function";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Function);
}

#[test]
fn test_keyword_heap() {
    let input = "heap";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Heap);
}

#[test]
fn test_keyword_high() {
    let input = "high";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::High);
}

#[test]
fn test_keyword_if() {
    let input = "if";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::If);
}

#[test]
fn test_keyword_import() {
    let input = "import";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Import);
}

#[test]
fn test_keyword_in() {
    let input = "in";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::In);
}

#[test]
fn test_keyword_invariant() {
    let input = "invariant";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Invariant);
}

#[test]
fn test_keyword_is() {
    let input = "is";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Is);
}

#[test]
fn test_keyword_let() {
    let input = "let";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Let);
}

#[test]
fn test_keyword_loop() {
    let input = "loop";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Loop);
}

#[test]
fn test_keyword_low() {
    let input = "low";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Low);
}

#[test]
fn test_keyword_manual() {
    let input = "manual";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Manual);
}

#[test]
fn test_keyword_match() {
    let input = "match";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Match);
}

#[test]
fn test_keyword_module() {
    let input = "module";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Module);
}

#[test]
fn test_keyword_not() {
    let input = "not";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Not);
}

#[test]
fn test_keyword_nothing() {
    let input = "nothing";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Nothing);
}

#[test]
fn test_keyword_or() {
    let input = "or";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Or);
}

#[test]
fn test_keyword_program() {
    let input = "program";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Program);
}

#[test]
fn test_keyword_requires() {
    let input = "requires";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Requires);
}

#[test]
fn test_keyword_return() {
    let input = "return";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Return);
}

#[test]
fn test_keyword_shared() {
    let input = "shared";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Shared);
}

#[test]
fn test_keyword_stack() {
    let input = "stack";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Stack);
}

#[test]
fn test_keyword_stealth() {
    let input = "stealth";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Stealth);
}

#[test]
fn test_keyword_switch() {
    let input = "switch";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Switch);
}

#[test]
fn test_keyword_this() {
    let input = "this";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::This);
}

#[test]
fn test_keyword_true() {
    let input = "true";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::True);
}

#[test]
fn test_keyword_unknown() {
    let input = "unknown";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Unknown);
}

#[test]
fn test_keyword_watch() {
    let input = "watch";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::Watch);
}

#[test]
fn test_keyword_when() {
    let input = "when";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::When);
}

#[test]
fn test_keyword_while() {
    let input = "while";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens[0].kind, TokenKind::While);
}

// Test arithmetic operators
#[test]
fn test_operators_arithmetic() {
    let input = "+ - * / %";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 6); // 5 operators + EOF
    assert_eq!(tokens[0].kind, TokenKind::Plus);
    assert_eq!(tokens[1].kind, TokenKind::Minus);
    assert_eq!(tokens[2].kind, TokenKind::Star);
    assert_eq!(tokens[3].kind, TokenKind::Slash);
    assert_eq!(tokens[4].kind, TokenKind::Percent);
}

// Test bitwise operators
#[test]
fn test_operators_bitwise() {
    let input = "& | ^ ~ << >>";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 7); // 6 operators + EOF
    assert_eq!(tokens[0].kind, TokenKind::Ampersand);
    assert_eq!(tokens[1].kind, TokenKind::Pipe);
    assert_eq!(tokens[2].kind, TokenKind::Caret);
    assert_eq!(tokens[3].kind, TokenKind::Tilde);
    assert_eq!(tokens[4].kind, TokenKind::LeftShift);
    assert_eq!(tokens[5].kind, TokenKind::RightShift);
}

// Test comparison operators (excluding equality)
#[test]
fn test_operators_comparison() {
    let input = "< > <= >=";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 5); // 4 operators + EOF
    assert_eq!(tokens[0].kind, TokenKind::Less);
    assert_eq!(tokens[1].kind, TokenKind::Greater);
    assert_eq!(tokens[2].kind, TokenKind::LessEqual);
    assert_eq!(tokens[3].kind, TokenKind::GreaterEqual);
}

// Test assignment operators
#[test]
fn test_operators_assignment() {
    let input = "= += -= *= /= %=";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 7); // 6 operators + EOF
    assert_eq!(tokens[0].kind, TokenKind::Equal);
    assert_eq!(tokens[1].kind, TokenKind::PlusEqual);
    assert_eq!(tokens[2].kind, TokenKind::MinusEqual);
    assert_eq!(tokens[3].kind, TokenKind::StarEqual);
    assert_eq!(tokens[4].kind, TokenKind::SlashEqual);
    assert_eq!(tokens[5].kind, TokenKind::PercentEqual);
}

// Test range operator
#[test]
fn test_operator_range() {
    let input = "..";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].kind, TokenKind::DotDot);
    assert_eq!(tokens[0].lexeme, "..");
}

// Test punctuation
#[test]
fn test_punctuation() {
    let input = "( ) { } [ ] , ; : . ? @";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 13); // 12 punctuation + EOF
    assert_eq!(tokens[0].kind, TokenKind::LeftParen);
    assert_eq!(tokens[1].kind, TokenKind::RightParen);
    assert_eq!(tokens[2].kind, TokenKind::LeftBrace);
    assert_eq!(tokens[3].kind, TokenKind::RightBrace);
    assert_eq!(tokens[4].kind, TokenKind::LeftBracket);
    assert_eq!(tokens[5].kind, TokenKind::RightBracket);
    assert_eq!(tokens[6].kind, TokenKind::Comma);
    assert_eq!(tokens[7].kind, TokenKind::Semicolon);
    assert_eq!(tokens[8].kind, TokenKind::Colon);
    assert_eq!(tokens[9].kind, TokenKind::Dot);
    assert_eq!(tokens[10].kind, TokenKind::Question);
    assert_eq!(tokens[11].kind, TokenKind::At);
}

// Test line comments
#[test]
fn test_line_comment() {
    let input = "42 // this is a comment\n84";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 3); // 42, 84, EOF
    assert_eq!(tokens[0].kind, TokenKind::Integer(42));
    assert_eq!(tokens[1].kind, TokenKind::Integer(84));
}

// Test block comments
#[test]
fn test_block_comment() {
    let input = "42 /* this is a comment */ 84";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 3); // 42, 84, EOF
    assert_eq!(tokens[0].kind, TokenKind::Integer(42));
    assert_eq!(tokens[1].kind, TokenKind::Integer(84));
}

// Test nested block comments
#[test]
fn test_nested_block_comment() {
    let input = "42 /* outer /* inner */ still in outer */ 84";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 3); // 42, 84, EOF
    assert_eq!(tokens[0].kind, TokenKind::Integer(42));
    assert_eq!(tokens[1].kind, TokenKind::Integer(84));
}

// Test position tracking
#[test]
fn test_position_tracking() {
    let input = "let x\n= 42";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 5); // let, x, =, 42, EOF

    // let at line 1, column 1
    assert_eq!(tokens[0].position.line, 1);
    assert_eq!(tokens[0].position.column, 1);

    // x at line 1, column 5
    assert_eq!(tokens[1].position.line, 1);
    assert_eq!(tokens[1].position.column, 5);

    // = at line 2, column 1
    assert_eq!(tokens[2].position.line, 2);
    assert_eq!(tokens[2].position.column, 1);

    // 42 at line 2, column 3
    assert_eq!(tokens[3].position.line, 2);
    assert_eq!(tokens[3].position.column, 3);
}

// Phase 1b: Equality operators and negation comparators tests

#[test]
fn test_equality_operator() {
    let input = "x ?= y";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 4); // x, ?=, y, EOF
    assert_eq!(tokens[0].kind, TokenKind::Identifier);
    assert_eq!(tokens[0].lexeme, "x");
    assert_eq!(tokens[1].kind, TokenKind::EqStrict);
    assert_eq!(tokens[1].lexeme, "?=");
    assert_eq!(tokens[2].kind, TokenKind::Identifier);
    assert_eq!(tokens[2].lexeme, "y");
}

#[test]
fn test_inequality_operator() {
    let input = "x != y";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 4); // x, !=, y, EOF
    assert_eq!(tokens[0].kind, TokenKind::Identifier);
    assert_eq!(tokens[0].lexeme, "x");
    assert_eq!(tokens[1].kind, TokenKind::NotEq);
    assert_eq!(tokens[1].lexeme, "!=");
    assert_eq!(tokens[2].kind, TokenKind::Identifier);
    assert_eq!(tokens[2].lexeme, "y");
}

#[test]
fn test_not_less_operator() {
    let input = "x !< y";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 4); // x, !<, y, EOF
    assert_eq!(tokens[0].kind, TokenKind::Identifier);
    assert_eq!(tokens[0].lexeme, "x");
    assert_eq!(tokens[1].kind, TokenKind::NotLess);
    assert_eq!(tokens[1].lexeme, "!<");
    assert_eq!(tokens[2].kind, TokenKind::Identifier);
    assert_eq!(tokens[2].lexeme, "y");
}

#[test]
fn test_not_greater_operator() {
    let input = "x !> y";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 4); // x, !>, y, EOF
    assert_eq!(tokens[0].kind, TokenKind::Identifier);
    assert_eq!(tokens[0].lexeme, "x");
    assert_eq!(tokens[1].kind, TokenKind::NotGreater);
    assert_eq!(tokens[1].lexeme, "!>");
    assert_eq!(tokens[2].kind, TokenKind::Identifier);
    assert_eq!(tokens[2].lexeme, "y");
}

#[test]
fn test_question_token_alone() {
    let input = "x ? y";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 4); // x, ?, y, EOF
    assert_eq!(tokens[0].kind, TokenKind::Identifier);
    assert_eq!(tokens[0].lexeme, "x");
    assert_eq!(tokens[1].kind, TokenKind::Question);
    assert_eq!(tokens[1].lexeme, "?");
    assert_eq!(tokens[2].kind, TokenKind::Identifier);
    assert_eq!(tokens[2].lexeme, "y");
}

#[test]
fn test_less_equal_regression() {
    let input = "x <= y";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 4); // x, <=, y, EOF
    assert_eq!(tokens[0].kind, TokenKind::Identifier);
    assert_eq!(tokens[1].kind, TokenKind::LessEqual);
    assert_eq!(tokens[1].lexeme, "<=");
    assert_eq!(tokens[2].kind, TokenKind::Identifier);
}

#[test]
fn test_greater_equal_regression() {
    let input = "x >= y";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 4); // x, >=, y, EOF
    assert_eq!(tokens[0].kind, TokenKind::Identifier);
    assert_eq!(tokens[1].kind, TokenKind::GreaterEqual);
    assert_eq!(tokens[1].lexeme, ">=");
    assert_eq!(tokens[2].kind, TokenKind::Identifier);
}

#[test]
fn test_double_equals_error() {
    let input = "x == y";
    let result = Lexer::new(input).tokens();
    assert!(result.is_err());
    if let Err(error) = result {
        match error {
            hilowc::lexer::LexError::InvalidOperator { suggestion, .. } => {
                assert!(suggestion.contains("use '?='"));
            }
            _ => panic!("Expected InvalidOperator error"),
        }
    }
}

#[test]
fn test_not_less_equal_error() {
    let input = "x !<= y";
    let result = Lexer::new(input).tokens();
    assert!(result.is_err());
    if let Err(error) = result {
        match error {
            hilowc::lexer::LexError::InvalidOperator { suggestion, .. } => {
                assert!(suggestion.contains("redundant"));
                assert!(suggestion.contains("'>' instead"));
            }
            _ => panic!("Expected InvalidOperator error"),
        }
    }
}

#[test]
fn test_not_greater_equal_error() {
    let input = "x !>= y";
    let result = Lexer::new(input).tokens();
    assert!(result.is_err());
    if let Err(error) = result {
        match error {
            hilowc::lexer::LexError::InvalidOperator { suggestion, .. } => {
                assert!(suggestion.contains("redundant"));
                assert!(suggestion.contains("'<' instead"));
            }
            _ => panic!("Expected InvalidOperator error"),
        }
    }
}

#[test]
fn test_bare_exclamation_error() {
    let input = "!flag";
    let result = Lexer::new(input).tokens();
    assert!(result.is_err());
    if let Err(error) = result {
        match error {
            hilowc::lexer::LexError::InvalidOperator { suggestion, .. } => {
                assert!(suggestion.contains("not"));
            }
            _ => panic!("Expected InvalidOperator error"),
        }
    }
}

#[test]
fn test_tilde_regression() {
    let input = "~x";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 3); // ~, x, EOF
    assert_eq!(tokens[0].kind, TokenKind::Tilde);
    assert_eq!(tokens[0].lexeme, "~");
    assert_eq!(tokens[1].kind, TokenKind::Identifier);
    assert_eq!(tokens[1].lexeme, "x");
}

#[test]
fn test_result_is_unknown() {
    let input = "result is unknown";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 4); // result, is, unknown, EOF
    assert_eq!(tokens[0].kind, TokenKind::Identifier);
    assert_eq!(tokens[0].lexeme, "result");
    assert_eq!(tokens[1].kind, TokenKind::Is);
    assert_eq!(tokens[1].lexeme, "is");
    assert_eq!(tokens[2].kind, TokenKind::Unknown);
    assert_eq!(tokens[2].lexeme, "unknown");
}

// Phase 6a: String literal tests

#[test]
fn test_simple_string() {
    let input = r#""hello""#;
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 2); // string, EOF
    assert_eq!(tokens[0].kind, TokenKind::StringLit("hello".to_string()));
    assert_eq!(tokens[0].lexeme, r#""hello""#);
}

#[test]
fn test_double_quote_recursion() {
    let input = r#"""contains "quotes" inside"""#;
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].kind, TokenKind::StringLit(r#"contains "quotes" inside"#.to_string()));
}

#[test]
fn test_triple_quote_recursion() {
    let input = r#""""triple-quoted""""#;
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].kind, TokenKind::StringLit("triple-quoted".to_string()));
}

#[test]
fn test_string_with_escapes() {
    let input = r#""hello\nworld""#;
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].kind, TokenKind::StringLit("hello\nworld".to_string()));
}

#[test]
fn test_raw_string_simple() {
    let input = r#"r"C:\Users\Alice""#;
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].kind, TokenKind::StringLit(r"C:\Users\Alice".to_string()));
}

#[test]
fn test_raw_string_no_escape_processing() {
    let input = r#"r"\n\t""#;
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].kind, TokenKind::StringLit(r"\n\t".to_string()));
}

#[test]
fn test_unicode_escape() {
    let input = r#""\u{1F600}""#;
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 2);
    // U+1F600 is the grinning face emoji 😀
    assert_eq!(tokens[0].kind, TokenKind::StringLit("😀".to_string()));
}

#[test]
fn test_hex_escape() {
    let input = r#""\x41""#;
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].kind, TokenKind::StringLit("A".to_string()));
}

#[test]
fn test_multiline_string() {
    let input = "\"line1\nline2\nline3\"";
    let tokens = Lexer::new(input).tokens().unwrap();
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].kind, TokenKind::StringLit("line1\nline2\nline3".to_string()));
}

#[test]
fn test_unterminated_string_error() {
    let input = r#""unterminated"#;
    let result = Lexer::new(input).tokens();
    assert!(result.is_err());
    if let Err(error) = result {
        match error {
            hilowc::lexer::LexError::UnterminatedString { .. } => {
                // Expected error
            }
            _ => panic!("Expected UnterminatedString error, got {:?}", error),
        }
    }
}

#[test]
fn test_invalid_escape_sequence() {
    let input = r#""\q""#;
    let result = Lexer::new(input).tokens();
    assert!(result.is_err());
    if let Err(error) = result {
        match error {
            hilowc::lexer::LexError::InvalidEscapeSequence { sequence, .. } => {
                assert_eq!(sequence, r"\q");
            }
            _ => panic!("Expected InvalidEscapeSequence error, got {:?}", error),
        }
    }
}