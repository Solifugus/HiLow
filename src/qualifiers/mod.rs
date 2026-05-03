use crate::types::Type;

#[derive(Debug, Clone, PartialEq)]
pub enum QualifierContext {
    Assignment,
    Equality,
    Both,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ArgSpec {
    None,                    // No arguments allowed
    SingleNamed,            // Single named argument required: (within: 0.01)
    SingleOptional,         // Single argument optional
}

#[derive(Debug, Clone, PartialEq)]
pub enum CodegenStatus {
    Implemented,
    NotYetForType(String), // Points to phase where it will be implemented
    NotYetImplemented(String), // Points to phase where it will be implemented
}

#[derive(Debug, Clone)]
pub struct QualifierInfo {
    pub name: &'static str,
    pub contexts: QualifierContext,
    pub args: ArgSpec,
    pub codegen_status: CodegenStatus,
    pub applies_to_type: fn(&Type) -> bool,
}

impl QualifierInfo {
    pub fn new(
        name: &'static str,
        contexts: QualifierContext,
        args: ArgSpec,
        codegen_status: CodegenStatus,
        applies_to_type: fn(&Type) -> bool,
    ) -> Self {
        Self {
            name,
            contexts,
            args,
            codegen_status,
            applies_to_type,
        }
    }
}

pub struct QualifierRegistry {
    qualifiers: Vec<QualifierInfo>,
}

impl QualifierRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            qualifiers: Vec::new(),
        };

        registry.register_builtin_qualifiers();
        registry
    }

    fn register_builtin_qualifiers(&mut self) {
        // Universal assignment qualifiers - implemented in Phase 5b
        self.qualifiers.push(QualifierInfo::new(
            "or",
            QualifierContext::Assignment,
            ArgSpec::None,
            CodegenStatus::Implemented,
            |t| matches!(t, Type::Bool),
        ));

        self.qualifiers.push(QualifierInfo::new(
            "and",
            QualifierContext::Assignment,
            ArgSpec::None,
            CodegenStatus::Implemented,
            |t| matches!(t, Type::Bool),
        ));

        self.qualifiers.push(QualifierInfo::new(
            "bitor",
            QualifierContext::Assignment,
            ArgSpec::None,
            CodegenStatus::Implemented,
            |t| matches!(t, Type::I8 | Type::I16 | Type::I32 |
                Type::I64 | Type::I128 | Type::Isize |
                Type::U8 | Type::U16 | Type::U32 |
                Type::U64 | Type::U128 | Type::Usize),
        ));

        self.qualifiers.push(QualifierInfo::new(
            "bitand",
            QualifierContext::Assignment,
            ArgSpec::None,
            CodegenStatus::Implemented,
            |t| matches!(t, Type::I8 | Type::I16 | Type::I32 |
                Type::I64 | Type::I128 | Type::Isize |
                Type::U8 | Type::U16 | Type::U32 |
                Type::U64 | Type::U128 | Type::Usize),
        ));

        self.qualifiers.push(QualifierInfo::new(
            "bitxor",
            QualifierContext::Assignment,
            ArgSpec::None,
            CodegenStatus::Implemented,
            |t| matches!(t, Type::I8 | Type::I16 | Type::I32 |
                Type::I64 | Type::I128 | Type::Isize |
                Type::U8 | Type::U16 | Type::U32 |
                Type::U64 | Type::U128 | Type::Usize),
        ));

        // Coercion qualifier - registered but not yet implemented
        self.qualifiers.push(QualifierInfo::new(
            "coerce",
            QualifierContext::Assignment,
            ArgSpec::None,
            CodegenStatus::NotYetImplemented("Phase 6c (strings) and Phase 9 (time, money)".to_string()),
            |_| true, // Will be type-specific once implemented
        ));

        // Future equality qualifiers - registered but not yet implemented
        self.qualifiers.push(QualifierInfo::new(
            "roughly",
            QualifierContext::Equality,
            ArgSpec::None,
            CodegenStatus::NotYetImplemented("Phase 9".to_string()),
            |t| matches!(t, Type::F32 | Type::F64),
        ));

        self.qualifiers.push(QualifierInfo::new(
            "caseless",
            QualifierContext::Equality,
            ArgSpec::None,
            CodegenStatus::NotYetImplemented("Phase 6c".to_string()),
            |t| matches!(t, Type::String),
        ));

        self.qualifiers.push(QualifierInfo::new(
            "trimmed",
            QualifierContext::Equality,
            ArgSpec::None,
            CodegenStatus::NotYetImplemented("Phase 6c".to_string()),
            |t| matches!(t, Type::String),
        ));
    }

    pub fn get_qualifier(&self, name: &str) -> Option<&QualifierInfo> {
        self.qualifiers.iter().find(|q| q.name == name)
    }

    pub fn is_valid_in_context(&self, name: &str, is_assignment: bool) -> bool {
        if let Some(qualifier) = self.get_qualifier(name) {
            match qualifier.contexts {
                QualifierContext::Assignment => is_assignment,
                QualifierContext::Equality => !is_assignment,
                QualifierContext::Both => true,
            }
        } else {
            false
        }
    }

    pub fn check_args(&self, name: &str, has_arg: bool) -> Result<(), String> {
        if let Some(qualifier) = self.get_qualifier(name) {
            match (&qualifier.args, has_arg) {
                (ArgSpec::None, true) => Err(format!("qualifier '{}' takes no arguments", name)),
                (ArgSpec::SingleNamed, false) => Err(format!("qualifier '{}' requires a named argument", name)),
                _ => Ok(()),
            }
        } else {
            Err(format!("qualifier '{}' is not defined", name))
        }
    }
}

impl Default for QualifierRegistry {
    fn default() -> Self {
        Self::new()
    }
}