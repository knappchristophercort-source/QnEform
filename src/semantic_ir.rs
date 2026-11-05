//! Versioned, backend-independent semantic IR schema.
//!
//! IDs are deterministic within one lowered module, but are not global IDs
//! and are not cryptographic identities. Source spans preserve provenance.
//! This schema records resource declarations without claiming allocation,
//! ownership transfer, release, or quantum execution.

use crate::{
    ast::Program,
    lexer::Span,
    semantic::{self, Domain, SemanticInstruction, ValueType},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SymbolId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScopeId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    Qubit,
    QubitRegister,
    CleanAncilla,
    DirtyAncilla,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ownership {
    Owned,
    Borrowed,
    Unassigned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceLifecycle {
    Declared,
    Allocated,
    Active,
    Uncomputed,
    VerifiedClean,
    Released,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntanglementStatus {
    KnownSeparable,
    PossiblyEntangled,
    KnownEntangled,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    PureClassical,
    QuantumResourceDeclaration,
    Measurement,
    Unitary,
    NonunitaryChannel,
    ResourceAllocation,
    ResourceRelease,
    ClassicalControl,
    ParameterUpdate,
    Observation,
    Suspend,
    Restore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProofObligationKind {
    ResourceOwnership,
    ResourceLifecycle,
    AncillaCleanliness,
    Reversibility,
    MeasurementSemantics,
    BackendConformance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofObligation {
    pub id: u32,
    pub kind: ProofObligationKind,
    pub subject_resource: Option<ResourceId>,
    pub required: bool,
    pub satisfied: bool,
    pub source: Option<Span>,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceOrigin {
    pub span: Span,
    pub source_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolRecord {
    pub id: SymbolId,
    pub name: String,
    pub value_type: ValueType,
    pub scope: ScopeId,
    pub domain: Domain,
    pub origin: SourceOrigin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceRecord {
    pub id: ResourceId,
    pub symbol: SymbolId,
    pub kind: ResourceKind,
    pub ownership: Ownership,
    pub lifecycle: ResourceLifecycle,
    pub entanglement: EntanglementStatus,
    pub owner_scope: ScopeId,
    pub origin: SourceOrigin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operand {
    Symbol(SymbolId),
    Resource(ResourceId),
    NumberLiteral(String),
    StringLiteral(String),
    BooleanLiteral(bool),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationKind {
    EnterDomain(Domain),
    ExitDomain(Domain),
    DeclareResource(ResourceId),
    Measure { target: ResourceId },
    Bind { target: SymbolId, value: Operand },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Operation {
    pub id: u32,
    pub kind: OperationKind,
    pub domain: Domain,
    pub effects: Vec<Effect>,
    pub origin: SourceOrigin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticIrModule {
    pub schema_version: String,
    pub module_id: String,
    pub source_name: Option<String>,
    pub symbols: Vec<SymbolRecord>,
    pub resources: Vec<ResourceRecord>,
    pub operations: Vec<Operation>,
    pub effects: Vec<Effect>,
    pub proof_obligations: Vec<ProofObligation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoweringError {
    pub message: String,
    pub span: Option<Span>,
}

impl std::fmt::Display for LoweringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.span {
            Some(span) => write!(f, "{} at {}:{}", self.message, span.line, span.column),
            None => write!(f, "{}", self.message),
        }
    }
}

impl std::error::Error for LoweringError {}

fn source_origin(span: Span, source_name: Option<&str>) -> SourceOrigin {
    SourceOrigin {
        span,
        source_name: source_name.map(str::to_owned),
    }
}

/// Lower an already semantically valid program into the versioned IR.
///
/// The caller must pass the validated semantic module generated from this
/// exact Program. This function checks symbol/instruction consistency but
/// cannot cryptographically bind the two objects; a future compilation
/// envelope will bind source and IR hashes.
pub fn lower(
    program: &Program,
    validated: &semantic::SemanticModule,
    source_name: Option<&str>,
) -> Result<SemanticIrModule, LoweringError> {
    let mut symbols = Vec::new();
    let mut resources = Vec::new();
    let mut operations = Vec::new();
    let mut all_effects = Vec::new();
    let mut symbol_ids = std::collections::HashMap::<String, SymbolId>::new();
    let mut resource_ids = std::collections::HashMap::<String, ResourceId>::new();
    let mut scope_stack = vec![ScopeId(0)];
    let mut next_scope = 1u32;
    let mut next_symbol = 0u32;
    let mut next_resource = 0u32;
    let mut next_operation = 0u32;

    fn walk(
        statements: &[crate::ast::Statement],
        domain: Domain,
        scope_stack: &mut Vec<ScopeId>,
        next_scope: &mut u32,
        next_symbol: &mut u32,
        next_resource: &mut u32,
        next_operation: &mut u32,
        symbols: &mut Vec<SymbolRecord>,
        resources: &mut Vec<ResourceRecord>,
        operations: &mut Vec<Operation>,
        symbol_ids: &mut std::collections::HashMap<String, SymbolId>,
        resource_ids: &mut std::collections::HashMap<String, ResourceId>,
        source_name: Option<&str>,
    ) -> Result<(), LoweringError> {
        use crate::ast::StatementKind;

        for statement in statements {
            match &statement.kind {
                StatementKind::QubitDeclaration { name } => {
                    let symbol = SymbolId(*next_symbol);
                    *next_symbol += 1;
                    let resource = ResourceId(*next_resource);
                    *next_resource += 1;
                    let scope = *scope_stack.last().expect("root scope exists");
                    symbol_ids.insert(name.clone(), symbol);
                    resource_ids.insert(name.clone(), resource);
                    symbols.push(SymbolRecord {
                        id: symbol,
                        name: name.clone(),
                        value_type: ValueType::Qubit,
                        scope,
                        domain,
                        origin: source_origin(statement.span, source_name),
                    });
                    resources.push(ResourceRecord {
                        id: resource,
                        symbol,
                        kind: ResourceKind::Qubit,
                        ownership: Ownership::Unassigned,
                        lifecycle: ResourceLifecycle::Declared,
                        entanglement: EntanglementStatus::Unknown,
                        owner_scope: scope,
                        origin: source_origin(statement.span, source_name),
                    });
                    operations.push(Operation {
                        id: *next_operation,
                        kind: OperationKind::DeclareResource(resource),
                        domain,
                        effects: vec![Effect::QuantumResourceDeclaration],
                        origin: source_origin(statement.span, source_name),
                    });
                    *next_operation += 1;
                }
                StatementKind::Measurement { target } => {
                    let resource = *resource_ids.get(target).ok_or_else(|| LoweringError {
                        message: format!("validated measurement target '{target}' is missing from lowering symbol map"),
                        span: Some(statement.span),
                    })?;
                    operations.push(Operation {
                        id: *next_operation,
                        kind: OperationKind::Measure { target: resource },
                        domain,
                        effects: vec![Effect::Measurement, Effect::Observation],
                        origin: source_origin(statement.span, source_name),
                    });
                    *next_operation += 1;
                }
                StatementKind::LetBinding { name, value } => {
                    let symbol = SymbolId(*next_symbol);
                    *next_symbol += 1;
                    let (value_type, operand) = match value {
                        crate::ast::Expression::Number(value) =>
                            (ValueType::Number, Operand::NumberLiteral(value.clone())),
                        crate::ast::Expression::String(value) =>
                            (ValueType::String, Operand::StringLiteral(value.clone())),
                        crate::ast::Expression::Boolean(value) =>
                            (ValueType::Boolean, Operand::BooleanLiteral(*value)),
                        crate::ast::Expression::Identifier(reference) => {
                            let referenced = *symbol_ids.get(reference).ok_or_else(|| LoweringError {
                                message: format!("validated reference '{reference}' is missing from lowering symbol map"),
                                span: Some(statement.span),
                            })?;
                            let record = symbols.iter().find(|record| record.id == referenced)
                                .ok_or_else(|| LoweringError {
                                    message: format!("symbol record for '{reference}' is missing"),
                                    span: Some(statement.span),
                                })?;
                            (record.value_type, Operand::Symbol(referenced))
                        }
                    };
                    let scope = *scope_stack.last().expect("root scope exists");
                    symbol_ids.insert(name.clone(), symbol);
                    symbols.push(SymbolRecord {
                        id: symbol,
                        name: name.clone(),
                        value_type,
                        scope,
                        domain,
                        origin: source_origin(statement.span, source_name),
                    });
                    operations.push(Operation {
                        id: *next_operation,
                        kind: OperationKind::Bind { target: symbol, value: operand },
                        domain,
                        effects: vec![Effect::PureClassical],
                        origin: source_origin(statement.span, source_name),
                    });
                    *next_operation += 1;
                }
                StatementKind::QuantumBlock { statements: nested } => {
                    let scope = ScopeId(*next_scope);
                    *next_scope += 1;
                    scope_stack.push(scope);
                    operations.push(Operation {
                        id: *next_operation,
                        kind: OperationKind::EnterDomain(Domain::Quantum),
                        domain: Domain::Quantum,
                        effects: Vec::new(),
                        origin: source_origin(statement.span, source_name),
                    });
                    *next_operation += 1;
                    walk(nested, Domain::Quantum, scope_stack, next_scope, next_symbol,
                        next_resource, next_operation, symbols, resources, operations,
                        symbol_ids, resource_ids, source_name)?;
                    operations.push(Operation {
                        id: *next_operation,
                        kind: OperationKind::ExitDomain(Domain::Quantum),
                        domain: Domain::Quantum,
                        effects: Vec::new(),
                        origin: source_origin(statement.span, source_name),
                    });
                    *next_operation += 1;
                    scope_stack.pop();
                }
                StatementKind::ClassicalBlock { statements: nested } => {
                    let scope = ScopeId(*next_scope);
                    *next_scope += 1;
                    scope_stack.push(scope);
                    operations.push(Operation {
                        id: *next_operation,
                        kind: OperationKind::EnterDomain(Domain::Classical),
                        domain: Domain::Classical,
                        effects: Vec::new(),
                        origin: source_origin(statement.span, source_name),
                    });
                    *next_operation += 1;
                    walk(nested, Domain::Classical, scope_stack, next_scope, next_symbol,
                        next_resource, next_operation, symbols, resources, operations,
                        symbol_ids, resource_ids, source_name)?;
                    operations.push(Operation {
                        id: *next_operation,
                        kind: OperationKind::ExitDomain(Domain::Classical),
                        domain: Domain::Classical,
                        effects: Vec::new(),
                        origin: source_origin(statement.span, source_name),
                    });
                    *next_operation += 1;
                    scope_stack.pop();
                }
            }
        }
        Ok(())
    }

    // Verify that the semantic instruction stream and AST have matching
    // statement counts before lowering. This is a consistency guard, not a
    // cryptographic provenance proof.
    let expected_non_block = validated.instructions.iter().filter(|instruction| matches!(
        instruction,
        SemanticInstruction::DeclareQubit { .. } |
        SemanticInstruction::Measure { .. } |
        SemanticInstruction::Bind { .. }
    )).count();
    let ast_non_block = count_non_block_statements(&program.statements);
    if expected_non_block != ast_non_block {
        return Err(LoweringError {
            message: "validated semantic module does not match the supplied AST".to_owned(),
            span: None,
        });
    }

    walk(
        &program.statements, Domain::Hybrid, &mut scope_stack, &mut next_scope,
        &mut next_symbol, &mut next_resource, &mut next_operation,
        &mut symbols, &mut resources, &mut operations,
        &mut symbol_ids, &mut resource_ids, source_name,
    )?;

    for operation in &operations {
        for effect in &operation.effects {
            if !all_effects.contains(effect) {
                all_effects.push(*effect);
            }
        }
    }

    let proof_obligations = resources.iter().map(|resource| ProofObligation {
        id: resource.id.0,
        kind: ProofObligationKind::ResourceOwnership,
        subject_resource: Some(resource.id),
        required: true,
        satisfied: false,
        source: Some(resource.origin.span),
        description: "Resource ownership is not yet assigned by the current source language.".to_owned(),
    }).collect();

    Ok(SemanticIrModule {
        schema_version: "qneform-ir-0.2".to_owned(),
        module_id: "module-0".to_owned(),
        source_name: source_name.map(str::to_owned),
        symbols,
        resources,
        operations,
        effects: all_effects,
        proof_obligations,
    })
}

fn count_non_block_statements(statements: &[crate::ast::Statement]) -> usize {
    use crate::ast::StatementKind;
    statements.iter().map(|statement| match &statement.kind {
        StatementKind::QuantumBlock { statements } |
        StatementKind::ClassicalBlock { statements } => count_non_block_statements(statements),
        _ => 1,
    }).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lexer::Lexer, parser::Parser};

    fn lower_source(source: &str) -> SemanticIrModule {
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse().unwrap();
        let validated = semantic::analyze(&program).unwrap();
        lower(&program, &validated, Some("test.qn")).unwrap()
    }

    #[test]
    fn lowers_symbols_resources_effects_and_provenance() {
        let ir = lower_source("qbit q0; quantum { measure q0; } classical { let shots := 10; }");
        assert_eq!(ir.schema_version, "qneform-ir-0.2");
        assert_eq!(ir.resources.len(), 1);
        assert_eq!(ir.resources[0].lifecycle, ResourceLifecycle::Declared);
        assert_eq!(ir.resources[0].ownership, Ownership::Unassigned);
        assert!(ir.effects.contains(&Effect::Measurement));
        assert!(ir.symbols.iter().all(|symbol| symbol.origin.source_name.as_deref() == Some("test.qn")));
    }

    #[test]
    fn resource_proof_obligation_is_explicitly_unsatisfied() {
        let ir = lower_source("qbit q0;");
        assert_eq!(ir.proof_obligations.len(), 1);
        assert!(!ir.proof_obligations[0].satisfied);
    }

    #[test]
    fn nested_scope_gets_distinct_scope_id() {
        let ir = lower_source("classical { let count := 10; }");
        assert_ne!(ir.symbols[0].scope, ScopeId(0));
    }

    #[test]
    fn module_local_ids_are_deterministic_for_same_input() {
        let source = "qbit q0; measure q0;";
        assert_eq!(lower_source(source), lower_source(source));
    }

    #[test]
    fn rejects_mismatched_semantic_module_and_ast() {
        let a = Lexer::new("qbit q0;").tokenize().unwrap();
        let program_a = Parser::new(a).parse().unwrap();
        let validated_a = semantic::analyze(&program_a).unwrap();
        let b = Lexer::new("let x := 1;").tokenize().unwrap();
        let program_b = Parser::new(b).parse().unwrap();
        assert!(lower(&program_b, &validated_a, None).is_err());
    }
}
