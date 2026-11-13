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

/// A required proof obligation that blocks backend readiness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendReadinessIssue {
    pub obligation_id: u32,
    pub kind: ProofObligationKind,
    pub subject_resource: Option<ResourceId>,
    pub description: String,
}

/// All unresolved required proof obligations found during backend preflight.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendReadinessError {
    pub issues: Vec<BackendReadinessIssue>,
}

impl std::fmt::Display for BackendReadinessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "backend readiness blocked by {} unresolved required proof obligation(s)",
            self.issues.len()
        )?;
        for issue in &self.issues {
            write!(
                f,
                "\n- obligation {} ({:?}): {}",
                issue.obligation_id, issue.kind, issue.description
            )?;
        }
        Ok(())
    }
}

impl std::error::Error for BackendReadinessError {}

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
    AllocateResource(ResourceId),
    ReleaseResource(ResourceId),
    BorrowResource { target: ResourceId, alias: String },
    ReturnBorrow { alias: String },
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

/// Canonical, versioned binary representation of the semantic module.
///
/// This encoding intentionally excludes source paths and source spans: they
/// are provenance metadata, not executable semantic identity. Vector order is
/// preserved because operation and declaration order are semantically relevant.
/// Integer values use fixed-width big-endian encoding; strings are UTF-8 with
/// an unsigned 64-bit byte length prefix. Enum discriminants are explicit and
/// must never be reordered within this encoding version.
impl SemanticIrModule {
    /// SHA-256 identity of this module's versioned canonical semantic encoding.
    ///
    /// Source paths and spans are excluded by canonical_bytes(); semantic
    /// declarations, operations, effects and proof obligations are included.
    pub fn fingerprint(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let digest = Sha256::digest(self.canonical_bytes());
        let mut fingerprint = [0u8; 32];
        fingerprint.copy_from_slice(&digest);
        fingerprint
    }

    /// Lowercase hexadecimal representation suitable for logs and diagnostics.
    pub fn fingerprint_hex(&self) -> String {
        self.fingerprint()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    /// Reject backend preparation while any required proof obligation is unresolved.
    ///
    /// This is a fail-closed IR preflight check, not an execution authorization
    /// token or proof verifier. The IR currently exposes obligation status as
    /// mutable data; a production backend must additionally require trusted,
    /// provenance-bound verifier evidence before executing quantum work.
    /// Optional unresolved obligations do not block readiness.
    pub fn validate_backend_readiness(&self) -> Result<(), BackendReadinessError> {
        let issues = self
            .proof_obligations
            .iter()
            .filter(|obligation| obligation.required && !obligation.satisfied)
            .map(|obligation| BackendReadinessIssue {
                obligation_id: obligation.id,
                kind: obligation.kind,
                subject_resource: obligation.subject_resource,
                description: obligation.description.clone(),
            })
            .collect::<Vec<_>>();

        if issues.is_empty() {
            Ok(())
        } else {
            Err(BackendReadinessError { issues })
        }
    }

    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = b"QNEFORM-IR-CANON\0".to_vec();
        put_string(&mut out, "qneform-canonical-ir-v1");
        put_string(&mut out, &self.schema_version);
        put_string(&mut out, &self.module_id);

        put_len(&mut out, self.symbols.len());
        for item in &self.symbols {
            put_u32(&mut out, item.id.0);
            put_string(&mut out, &item.name);
            put_u8(&mut out, value_type_tag(item.value_type));
            put_u32(&mut out, item.scope.0);
            put_u8(&mut out, domain_tag(item.domain));
        }

        put_len(&mut out, self.resources.len());
        for item in &self.resources {
            put_u32(&mut out, item.id.0);
            put_u32(&mut out, item.symbol.0);
            put_u8(&mut out, resource_kind_tag(item.kind));
            put_u8(&mut out, ownership_tag(item.ownership));
            put_u8(&mut out, lifecycle_tag(item.lifecycle));
            put_u8(&mut out, entanglement_tag(item.entanglement));
            put_u32(&mut out, item.owner_scope.0);
        }

        put_len(&mut out, self.operations.len());
        for item in &self.operations {
            put_u32(&mut out, item.id);
            put_u8(&mut out, domain_tag(item.domain));
            put_len(&mut out, item.effects.len());
            for effect in &item.effects {
                put_u8(&mut out, effect_tag(*effect));
            }
            match &item.kind {
                OperationKind::EnterDomain(domain) => {
                    put_u8(&mut out, 0);
                    put_u8(&mut out, domain_tag(*domain));
                }
                OperationKind::ExitDomain(domain) => {
                    put_u8(&mut out, 1);
                    put_u8(&mut out, domain_tag(*domain));
                }
                OperationKind::DeclareResource(id) => {
                    put_u8(&mut out, 2);
                    put_u32(&mut out, id.0);
                }
                OperationKind::AllocateResource(id) => {
                    put_u8(&mut out, 3);
                    put_u32(&mut out, id.0);
                }
                OperationKind::ReleaseResource(id) => {
                    put_u8(&mut out, 4);
                    put_u32(&mut out, id.0);
                }
                OperationKind::BorrowResource { target, alias } => {
                    put_u8(&mut out, 5);
                    put_u32(&mut out, target.0);
                    put_string(&mut out, alias);
                }
                OperationKind::ReturnBorrow { alias } => {
                    put_u8(&mut out, 6);
                    put_string(&mut out, alias);
                }
                OperationKind::Measure { target } => {
                    put_u8(&mut out, 7);
                    put_u32(&mut out, target.0);
                }
                OperationKind::Bind { target, value } => {
                    put_u8(&mut out, 8);
                    put_u32(&mut out, target.0);
                    match value {
                        Operand::Symbol(id) => {
                            put_u8(&mut out, 0);
                            put_u32(&mut out, id.0);
                        }
                        Operand::Resource(id) => {
                            put_u8(&mut out, 1);
                            put_u32(&mut out, id.0);
                        }
                        Operand::NumberLiteral(value) => {
                            put_u8(&mut out, 2);
                            put_string(&mut out, value);
                        }
                        Operand::StringLiteral(value) => {
                            put_u8(&mut out, 3);
                            put_string(&mut out, value);
                        }
                        Operand::BooleanLiteral(value) => {
                            put_u8(&mut out, 4);
                            put_u8(&mut out, u8::from(*value));
                        }
                    }
                }
            }
        }

        put_len(&mut out, self.effects.len());
        for effect in &self.effects {
            put_u8(&mut out, effect_tag(*effect));
        }

        put_len(&mut out, self.proof_obligations.len());
        for item in &self.proof_obligations {
            put_u32(&mut out, item.id);
            put_u8(&mut out, obligation_tag(item.kind));
            match item.subject_resource {
                Some(id) => {
                    put_u8(&mut out, 1);
                    put_u32(&mut out, id.0);
                }
                None => put_u8(&mut out, 0),
            }
            put_u8(&mut out, u8::from(item.required));
            put_u8(&mut out, u8::from(item.satisfied));
            put_string(&mut out, &item.description);
        }
        out
    }
}

fn put_u8(out: &mut Vec<u8>, value: u8) {
    out.push(value);
}

fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn put_len(out: &mut Vec<u8>, value: usize) {
    out.extend_from_slice(&(value as u64).to_be_bytes());
}

fn put_string(out: &mut Vec<u8>, value: &str) {
    put_len(out, value.len());
    out.extend_from_slice(value.as_bytes());
}

fn domain_tag(value: Domain) -> u8 {
    match value {
        Domain::Hybrid => 0,
        Domain::Quantum => 1,
        Domain::Classical => 2,
    }
}

fn value_type_tag(value: ValueType) -> u8 {
    match value {
        ValueType::Qubit => 0,
        ValueType::Number => 1,
        ValueType::String => 2,
        ValueType::Boolean => 3,
    }
}

fn resource_kind_tag(value: ResourceKind) -> u8 {
    match value {
        ResourceKind::Qubit => 0,
        ResourceKind::QubitRegister => 1,
        ResourceKind::CleanAncilla => 2,
        ResourceKind::DirtyAncilla => 3,
    }
}

fn ownership_tag(value: Ownership) -> u8 {
    match value {
        Ownership::Owned => 0,
        Ownership::Borrowed => 1,
        Ownership::Unassigned => 2,
    }
}

fn lifecycle_tag(value: ResourceLifecycle) -> u8 {
    match value {
        ResourceLifecycle::Declared => 0,
        ResourceLifecycle::Allocated => 1,
        ResourceLifecycle::Active => 2,
        ResourceLifecycle::Uncomputed => 3,
        ResourceLifecycle::VerifiedClean => 4,
        ResourceLifecycle::Released => 5,
        ResourceLifecycle::Unknown => 6,
    }
}

fn entanglement_tag(value: EntanglementStatus) -> u8 {
    match value {
        EntanglementStatus::KnownSeparable => 0,
        EntanglementStatus::PossiblyEntangled => 1,
        EntanglementStatus::KnownEntangled => 2,
        EntanglementStatus::Unknown => 3,
    }
}

fn effect_tag(value: Effect) -> u8 {
    match value {
        Effect::PureClassical => 0,
        Effect::QuantumResourceDeclaration => 1,
        Effect::Measurement => 2,
        Effect::Unitary => 3,
        Effect::NonunitaryChannel => 4,
        Effect::ResourceAllocation => 5,
        Effect::ResourceRelease => 6,
        Effect::ClassicalControl => 7,
        Effect::ParameterUpdate => 8,
        Effect::Observation => 9,
        Effect::Suspend => 10,
        Effect::Restore => 11,
    }
}

fn obligation_tag(value: ProofObligationKind) -> u8 {
    match value {
        ProofObligationKind::ResourceOwnership => 0,
        ProofObligationKind::ResourceLifecycle => 1,
        ProofObligationKind::AncillaCleanliness => 2,
        ProofObligationKind::Reversibility => 3,
        ProofObligationKind::MeasurementSemantics => 4,
        ProofObligationKind::BackendConformance => 5,
    }
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
                StatementKind::Allocate { target } => {
                    let resource = *resource_ids.get(target).ok_or_else(|| LoweringError {
                        message: format!("validated allocation target '{target}' is missing from lowering symbol map"),
                        span: Some(statement.span),
                    })?;
                    if let Some(record) = resources.iter_mut().find(|record| record.id == resource)
                    {
                        record.ownership = Ownership::Owned;
                        record.lifecycle = ResourceLifecycle::Allocated;
                    }
                    operations.push(Operation {
                        id: *next_operation,
                        kind: OperationKind::AllocateResource(resource),
                        domain,
                        effects: vec![Effect::ResourceAllocation],
                        origin: source_origin(statement.span, source_name),
                    });
                    *next_operation += 1;
                }
                StatementKind::Release { target } => {
                    let resource = *resource_ids.get(target).ok_or_else(|| LoweringError {
                        message: format!("validated release target '{target}' is missing from lowering symbol map"),
                        span: Some(statement.span),
                    })?;
                    if let Some(record) = resources.iter_mut().find(|record| record.id == resource)
                    {
                        record.lifecycle = ResourceLifecycle::Released;
                    }
                    operations.push(Operation {
                        id: *next_operation,
                        kind: OperationKind::ReleaseResource(resource),
                        domain,
                        effects: vec![Effect::ResourceRelease],
                        origin: source_origin(statement.span, source_name),
                    });
                    *next_operation += 1;
                }
                StatementKind::Borrow { target, alias } => {
                    let resource = *resource_ids.get(target).ok_or_else(|| LoweringError {
                        message: format!("validated borrow target '{target}' is missing from lowering symbol map"),
                        span: Some(statement.span),
                    })?;
                    operations.push(Operation {
                        id: *next_operation,
                        kind: OperationKind::BorrowResource {
                            target: resource,
                            alias: alias.clone(),
                        },
                        domain,
                        effects: vec![],
                        origin: source_origin(statement.span, source_name),
                    });
                    *next_operation += 1;
                }
                StatementKind::ReturnBorrow { alias } => {
                    operations.push(Operation {
                        id: *next_operation,
                        kind: OperationKind::ReturnBorrow {
                            alias: alias.clone(),
                        },
                        domain,
                        effects: vec![],
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
                        crate::ast::Expression::Number(value) => {
                            (ValueType::Number, Operand::NumberLiteral(value.clone()))
                        }
                        crate::ast::Expression::String(value) => {
                            (ValueType::String, Operand::StringLiteral(value.clone()))
                        }
                        crate::ast::Expression::Boolean(value) => {
                            (ValueType::Boolean, Operand::BooleanLiteral(*value))
                        }
                        crate::ast::Expression::Identifier(reference) => {
                            let referenced = *symbol_ids.get(reference).ok_or_else(|| LoweringError {
                                message: format!("validated reference '{reference}' is missing from lowering symbol map"),
                                span: Some(statement.span),
                            })?;
                            let record = symbols
                                .iter()
                                .find(|record| record.id == referenced)
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
                        kind: OperationKind::Bind {
                            target: symbol,
                            value: operand,
                        },
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
                    walk(
                        nested,
                        Domain::Quantum,
                        scope_stack,
                        next_scope,
                        next_symbol,
                        next_resource,
                        next_operation,
                        symbols,
                        resources,
                        operations,
                        symbol_ids,
                        resource_ids,
                        source_name,
                    )?;
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
                    walk(
                        nested,
                        Domain::Classical,
                        scope_stack,
                        next_scope,
                        next_symbol,
                        next_resource,
                        next_operation,
                        symbols,
                        resources,
                        operations,
                        symbol_ids,
                        resource_ids,
                        source_name,
                    )?;
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
    let expected_non_block = validated
        .instructions
        .iter()
        .filter(|instruction| {
            matches!(
                instruction,
                SemanticInstruction::DeclareQubit { .. }
                    | SemanticInstruction::Allocate { .. }
                    | SemanticInstruction::Release { .. }
                    | SemanticInstruction::Borrow { .. }
                    | SemanticInstruction::ReturnBorrow { .. }
                    | SemanticInstruction::Measure { .. }
                    | SemanticInstruction::Bind { .. }
            )
        })
        .count();
    let ast_non_block = count_non_block_statements(&program.statements);
    if expected_non_block != ast_non_block {
        return Err(LoweringError {
            message: "validated semantic module does not match the supplied AST".to_owned(),
            span: None,
        });
    }

    walk(
        &program.statements,
        Domain::Hybrid,
        &mut scope_stack,
        &mut next_scope,
        &mut next_symbol,
        &mut next_resource,
        &mut next_operation,
        &mut symbols,
        &mut resources,
        &mut operations,
        &mut symbol_ids,
        &mut resource_ids,
        source_name,
    )?;

    for operation in &operations {
        for effect in &operation.effects {
            if !all_effects.contains(effect) {
                all_effects.push(*effect);
            }
        }
    }

    let proof_obligations = resources
        .iter()
        .map(|resource| ProofObligation {
            id: resource.id.0,
            kind: ProofObligationKind::ResourceOwnership,
            subject_resource: Some(resource.id),
            required: true,
            satisfied: false,
            source: Some(resource.origin.span),
            description: "Resource ownership is not yet assigned by the current source language."
                .to_owned(),
        })
        .collect();

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
    statements
        .iter()
        .map(|statement| match &statement.kind {
            StatementKind::QuantumBlock { statements }
            | StatementKind::ClassicalBlock { statements } => {
                count_non_block_statements(statements)
            }
            _ => 1,
        })
        .sum()
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
        let ir = lower_source(
            "qbit q0; allocate q0; quantum { measure q0; } classical { let shots := 10; }",
        );
        assert_eq!(ir.schema_version, "qneform-ir-0.2");
        assert_eq!(ir.resources.len(), 1);
        assert_eq!(ir.resources[0].lifecycle, ResourceLifecycle::Allocated);
        assert_eq!(ir.resources[0].ownership, Ownership::Owned);
        assert!(ir.effects.contains(&Effect::Measurement));
        assert!(ir
            .symbols
            .iter()
            .all(|symbol| symbol.origin.source_name.as_deref() == Some("test.qn")));
    }

    #[test]
    fn backend_readiness_rejects_each_unresolved_required_obligation() {
        let mut ir = lower_source("qbit q0;");
        ir.proof_obligations.push(ProofObligation {
            id: 99,
            kind: ProofObligationKind::BackendConformance,
            subject_resource: None,
            required: true,
            satisfied: false,
            source: None,
            description: "backend profile has not been verified".to_owned(),
        });

        let error = ir.validate_backend_readiness().unwrap_err();
        assert_eq!(error.issues.len(), 2);
        assert_eq!(error.issues[0].obligation_id, 0);
        assert_eq!(error.issues[0].kind, ProofObligationKind::ResourceOwnership);
        assert_eq!(error.issues[1].obligation_id, 99);
        assert_eq!(error.issues[1].kind, ProofObligationKind::BackendConformance);
    }

    #[test]
    fn backend_readiness_allows_optional_unresolved_obligations() {
        let mut ir = lower_source("let shots := 10;");
        ir.proof_obligations.push(ProofObligation {
            id: 7,
            kind: ProofObligationKind::MeasurementSemantics,
            subject_resource: None,
            required: false,
            satisfied: false,
            source: None,
            description: "optional analysis not performed".to_owned(),
        });
        assert_eq!(ir.validate_backend_readiness(), Ok(()));
    }

    #[test]
    fn backend_readiness_passes_when_all_required_obligations_are_satisfied() {
        let mut ir = lower_source("qbit q0;");
        for obligation in &mut ir.proof_obligations {
            obligation.satisfied = true;
        }
        assert_eq!(ir.validate_backend_readiness(), Ok(()));
    }

    #[test]
    fn backend_readiness_passes_when_no_obligations_exist() {
        let ir = lower_source("let shots := 10;");
        assert!(ir.proof_obligations.is_empty());
        assert_eq!(ir.validate_backend_readiness(), Ok(()));
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
        let source = "qbit q0; allocate q0; measure q0;";
        assert_eq!(lower_source(source), lower_source(source));
    }

    #[test]
    fn canonical_encoding_is_stable_and_excludes_source_provenance() {
        let mut first = lower_source("qbit q0; allocate q0; measure q0;");
        let mut second = first.clone();
        second.source_name = Some("different/path.qn".to_owned());
        second.symbols[0].origin.source_name = Some("different/path.qn".to_owned());
        second.symbols[0].origin.span.line += 20;
        assert_eq!(first.canonical_bytes(), second.canonical_bytes());
        assert_eq!(first.canonical_bytes(), first.clone().canonical_bytes());
    }

    #[test]
    fn canonical_encoding_changes_when_semantics_change() {
        let first = lower_source("let shots := 10;");
        let second = lower_source("let shots := 11;");
        assert_ne!(first.canonical_bytes(), second.canonical_bytes());
    }

    #[test]
    fn sha256_implementation_passes_known_answer_and_module_fingerprint_is_32_bytes() {
        use sha2::{Digest, Sha256};
        let known_answer = Sha256::digest(b"abc");
        let expected = [
            0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
            0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61,
            0xf2, 0x00, 0x15, 0xad,
        ];
        assert_eq!(known_answer.as_slice(), expected);

        let ir = lower_source("let shots := 10;");
        assert_eq!(ir.fingerprint().len(), 32);
        assert_eq!(
            ir.fingerprint().as_slice(),
            Sha256::digest(ir.canonical_bytes()).as_slice()
        );
        assert_eq!(ir.fingerprint_hex().len(), 64);
    }

    #[test]
    fn fingerprint_is_stable_and_ignores_source_provenance() {
        let first = lower_source("qbit q0; allocate q0; measure q0;");
        let mut second = first.clone();
        second.source_name = Some("other.qn".to_owned());
        second.symbols[0].origin.source_name = Some("other.qn".to_owned());
        second.symbols[0].origin.span.line += 1;
        assert_eq!(first.fingerprint(), second.fingerprint());
    }

    #[test]
    fn fingerprint_changes_when_semantic_content_changes() {
        let first = lower_source("let shots := 10;");
        let second = lower_source("let shots := 11;");
        assert_ne!(first.fingerprint(), second.fingerprint());
    }

    #[test]
    fn rejects_mismatched_semantic_module_and_ast() {
        let a = Lexer::new("qbit q0;").tokenize().unwrap();
        let program_a = Parser::new(a).parse().unwrap();
        let validated_a = semantic::analyze(&program_a).unwrap();
        let b = Lexer::new("let x := 1; let y := 2;").tokenize().unwrap();
        let program_b = Parser::new(b).parse().unwrap();
        assert!(lower(&program_b, &validated_a, None).is_err());
    }
}
