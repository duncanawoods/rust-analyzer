//! This files contains the declaration of diagnostics kinds for ty and path lowering.

use hir_def::type_ref::TypeRefId;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TyLoweringDiagnostic {
    pub source: TypeRefId,
    pub kind: TyLoweringDiagnosticKind,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TyLoweringDiagnosticKind {
    PathDiagnostic(PathLoweringDiagnostic),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericArgsProhibitedReason {
    Module,
    TyParam,
    SelfTy,
    PrimitiveTy,
    Const,
    Static,
    /// When there is a generic enum, within the expression `Enum::Variant`,
    /// either `Enum` or `Variant` are allowed to have generic arguments, but not both.
    EnumVariant,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum PathLoweringDiagnostic {
    GenericArgsProhibited { segment: u32, reason: GenericArgsProhibitedReason },
    ParenthesizedGenericArgsWithoutFnTrait { segment: u32 },
}
