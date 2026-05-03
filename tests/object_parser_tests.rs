use hilowc::parser::Parser;
use hilowc::ast::*;

#[test]
fn test_empty_object_literal() {
    let input = "high program(): i32 { let x = {} }";
    let result = Parser::new(input).unwrap().parse();
    assert!(result.is_ok(), "Failed to parse empty object literal: {:?}", result);

    let top_level = result.unwrap();
    if let TopLevel::Program(program) = &top_level {
        if let Some(ProgramBody { items, .. }) = &program.body {
            if let BlockItem::Statement(Statement::Let(let_decl)) = &items[0] {
                if let Some(Expression::ObjectLiteral(obj_lit)) = &let_decl.initializer {
                    assert_eq!(obj_lit.properties.len(), 0, "Expected empty object");
                } else {
                    panic!("Expected ObjectLiteral in let initializer");
                }
            } else {
                panic!("Expected Let statement");
            }
        } else {
            panic!("Expected program body");
        }
    } else {
        panic!("Expected Program");
    }
}

#[test]
fn test_single_property_object_literal() {
    let input = "high program(): i32 { let point = { x: 10 } }";
    let result = Parser::new(input).unwrap().parse();
    assert!(result.is_ok(), "Failed to parse single property object: {:?}", result);

    let top_level = result.unwrap();
    if let TopLevel::Program(program) = &top_level {
        if let Some(ProgramBody { items, .. }) = &program.body {
            if let BlockItem::Statement(Statement::Let(let_decl)) = &items[0] {
                if let Some(Expression::ObjectLiteral(obj_lit)) = &let_decl.initializer {
                    assert_eq!(obj_lit.properties.len(), 1, "Expected one property");
                    assert_eq!(obj_lit.properties[0].0, "x");
                    if let Expression::IntLit(value, _) = &obj_lit.properties[0].1 {
                        assert_eq!(*value, 10);
                    } else {
                        panic!("Expected integer literal value");
                    }
                } else {
                    panic!("Expected ObjectLiteral in let initializer");
                }
            } else {
                panic!("Expected Let statement");
            }
        } else {
            panic!("Expected program body");
        }
    } else {
        panic!("Expected Program");
    }
}

#[test]
fn test_multiple_properties_object_literal() {
    let input = "high program(): i32 { let point = { x: 10, y: 20 } }";
    let result = Parser::new(input).unwrap().parse();
    assert!(result.is_ok(), "Failed to parse multiple properties object: {:?}", result);

    let top_level = result.unwrap();
    if let TopLevel::Program(program) = &top_level {
        if let Some(ProgramBody { items, .. }) = &program.body {
            if let BlockItem::Statement(Statement::Let(let_decl)) = &items[0] {
                if let Some(Expression::ObjectLiteral(obj_lit)) = &let_decl.initializer {
                    assert_eq!(obj_lit.properties.len(), 2, "Expected two properties");
                    assert_eq!(obj_lit.properties[0].0, "x");
                    assert_eq!(obj_lit.properties[1].0, "y");
                } else {
                    panic!("Expected ObjectLiteral in let initializer");
                }
            } else {
                panic!("Expected Let statement");
            }
        } else {
            panic!("Expected program body");
        }
    } else {
        panic!("Expected Program");
    }
}

#[test]
fn test_trailing_comma_object_literal() {
    let input = "high program(): i32 { let point = { x: 10, y: 20, } }";
    let result = Parser::new(input).unwrap().parse();
    assert!(result.is_ok(), "Failed to parse trailing comma object: {:?}", result);

    let top_level = result.unwrap();
    if let TopLevel::Program(program) = &top_level {
        if let Some(ProgramBody { items, .. }) = &program.body {
            if let BlockItem::Statement(Statement::Let(let_decl)) = &items[0] {
                if let Some(Expression::ObjectLiteral(obj_lit)) = &let_decl.initializer {
                    assert_eq!(obj_lit.properties.len(), 2, "Expected two properties with trailing comma");
                } else {
                    panic!("Expected ObjectLiteral in let initializer");
                }
            } else {
                panic!("Expected Let statement");
            }
        } else {
            panic!("Expected program body");
        }
    } else {
        panic!("Expected Program");
    }
}

#[test]
fn test_property_access_parsing() {
    let input = "high program(): i32 { let x = point.x }";
    let result = Parser::new(input).unwrap().parse();
    assert!(result.is_ok(), "Failed to parse property access: {:?}", result);

    let top_level = result.unwrap();
    if let TopLevel::Program(program) = &top_level {
        if let Some(ProgramBody { items, .. }) = &program.body {
            if let BlockItem::Statement(Statement::Let(let_decl)) = &items[0] {
                if let Some(Expression::MemberAccess(member_access)) = &let_decl.initializer {
                    assert_eq!(member_access.member, "x");
                    if let Expression::Ident(name, _) = &*member_access.object {
                        assert_eq!(name, "point");
                    } else {
                        panic!("Expected identifier in member access object");
                    }
                } else {
                    panic!("Expected MemberAccess in let initializer");
                }
            } else {
                panic!("Expected Let statement");
            }
        } else {
            panic!("Expected program body");
        }
    } else {
        panic!("Expected Program");
    }
}

#[test]
fn test_property_assignment_parsing() {
    let input = "high program(): i32 { point.x = 5 }";
    let result = Parser::new(input).unwrap().parse();
    assert!(result.is_ok(), "Failed to parse property assignment: {:?}", result);

    let top_level = result.unwrap();
    if let TopLevel::Program(program) = &top_level {
        if let Some(ProgramBody { items, .. }) = &program.body {
            if let BlockItem::Statement(Statement::Assign(assign_stmt)) = &items[0] {
                if let Expression::MemberAccess(member_access) = &assign_stmt.target {
                    assert_eq!(member_access.member, "x");
                    if let Expression::Ident(name, _) = &*member_access.object {
                        assert_eq!(name, "point");
                    } else {
                        panic!("Expected identifier in member access object");
                    }
                } else {
                    panic!("Expected MemberAccess in assignment target");
                }

                if let Expression::IntLit(value, _) = &assign_stmt.value {
                    assert_eq!(*value, 5);
                } else {
                    panic!("Expected integer literal in assignment value");
                }
            } else {
                panic!("Expected Assign statement");
            }
        } else {
            panic!("Expected program body");
        }
    } else {
        panic!("Expected Program");
    }
}