use hilowc::parser::Parser;
use hilowc::typecheck::TypeChecker;

// Helper function to parse and type check a program
fn type_check_program(input: &str) -> Result<(), Vec<hilowc::types::TypeError>> {
    let result = Parser::new(input).unwrap().parse();
    assert!(result.is_ok(), "Parse failed: {:?}", result);
    let top_level = result.unwrap();

    let mut type_checker = TypeChecker::new();
    type_checker.check(&top_level)
}

// Successful type checks

#[test]
fn test_empty_object_type_check() {
    let input = "high program(): i32 { let obj = {} }";
    let result = type_check_program(input);
    assert!(result.is_ok(), "Expected successful type check for empty object, got: {:?}", result);
}

#[test]
fn test_single_property_object_type_check() {
    let input = "high program(): i32 { let point = { x: 10 } }";
    let result = type_check_program(input);
    assert!(result.is_ok(), "Expected successful type check for single property object, got: {:?}", result);
}

#[test]
fn test_multiple_properties_object_type_check() {
    let input = "high program(): i32 { let point = { x: 10, y: 20 } }";
    let result = type_check_program(input);
    assert!(result.is_ok(), "Expected successful type check for multiple properties object, got: {:?}", result);
}

#[test]
fn test_mixed_types_object_type_check() {
    let input = "high program(): i32 { let data = { id: 42, name: \"Alice\", active: true } }";
    let result = type_check_program(input);
    assert!(result.is_ok(), "Expected successful type check for mixed types object, got: {:?}", result);
}

#[test]
fn test_property_access_type_check() {
    let input = "high program(): i32 {
        let point = { x: 10, y: 20 }
        let x_val = point.x
    }";
    let result = type_check_program(input);
    assert!(result.is_ok(), "Expected successful type check for property access, got: {:?}", result);
}

#[test]
fn test_property_assignment_type_check() {
    let input = "high program(): i32 {
        let point = { x: 10, y: 20 }
        point.x = 99
    }";
    let result = type_check_program(input);
    assert!(result.is_ok(), "Expected successful type check for property assignment, got: {:?}", result);
}

// Type errors

#[test]
fn test_property_access_on_non_object() {
    let input = "high program(): i32 {
        let x = 42
        let val = x.property
    }";
    let result = type_check_program(input);
    assert!(result.is_err(), "Expected type error for property access on non-object");

    if let Err(errors) = result {
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("Cannot access property 'property' on non-object type"));
    }
}

#[test]
fn test_property_access_nonexistent_property() {
    let input = "high program(): i32 {
        let point = { x: 10, y: 20 }
        let val = point.z
    }";
    let result = type_check_program(input);
    assert!(result.is_err(), "Expected type error for accessing nonexistent property");

    if let Err(errors) = result {
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("Object does not have property 'z'"));
        assert!(errors[0].message.contains("Phase 9 will allow runtime property access"));
    }
}

#[test]
fn test_property_assignment_type_mismatch() {
    let input = "high program(): i32 {
        let point = { x: 10, y: 20 }
        point.x = true
    }";
    let result = type_check_program(input);
    assert!(result.is_err(), "Expected type error for property assignment type mismatch");

    if let Err(errors) = result {
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("Cannot assign bool to i32"));
    }
}

#[test]
fn test_property_assignment_nonexistent_property() {
    let input = "high program(): i32 {
        let point = { x: 10, y: 20 }
        point.z = 30
    }";
    let result = type_check_program(input);
    assert!(result.is_err(), "Expected type error for assigning to nonexistent property");

    if let Err(errors) = result {
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("Object does not have property 'z'"));
        assert!(errors[0].message.contains("Phase 9 will allow runtime property access"));
    }
}

#[test]
fn test_nested_object_property_access() {
    let input = "high program(): i32 {
        let person = {
            name: \"Alice\",
            address: { street: \"123 Main St\", zip: 12345 }
        }
        let street = person.address.street
    }";
    let result = type_check_program(input);
    assert!(result.is_ok(), "Expected successful type check for nested object property access, got: {:?}", result);
}

#[test]
fn test_object_with_function_property() {
    // This test is for Phase 7a where functions are just values stored on objects
    // No special `this` binding yet - that's Phase 7c
    let input = "high program(): i32 {
        function greet(): string { return \"hello\" }
        let obj = { method: greet }
        let result = obj.method()
    }";
    let result = type_check_program(input);
    assert!(result.is_ok(), "Expected successful type check for object with function property, got: {:?}", result);
}