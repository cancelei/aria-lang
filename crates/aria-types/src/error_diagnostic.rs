//! Enhanced error diagnostics for TypeError.
//!
//! This module provides conversion from TypeError to rich Diagnostic messages
//! with context, suggestions, and color output.

use crate::{TypeError, TypeSource, BinaryOpSide};
use aria_diagnostics::{Diagnostic, suggestion::Suggestion};
use aria_diagnostics::span::SourceSpan;
use aria_lexer::Span;

impl TypeError {
    /// Convert this TypeError to a rich Diagnostic with context and suggestions.
    ///
    /// This is the main entry point for converting type errors into user-friendly
    /// diagnostics that leverage the aria-diagnostics infrastructure for colored
    /// output, code context, and fix suggestions.
    pub fn to_diagnostic(&self) -> Diagnostic {
        match self {
            // ============================================================================
            // Core Type Errors - Enhanced with context and suggestions
            // ============================================================================

            TypeError::Mismatch { expected, found, span, expected_source } => {
                let mut diag = Diagnostic::error(
                    "E0001",
                    "type mismatch"
                )
                .with_primary_span(
                    span_to_source_span(span),
                    format!("expected `{}`, found `{}`", expected, found)
                );

                // Add secondary span showing where the expected type came from
                if let Some(source) = expected_source {
                    match source {
                        TypeSource::Annotation(source_span) => {
                            diag = diag.with_secondary_span(
                                span_to_source_span(source_span),
                                format!("expected `{}` due to this type annotation", expected)
                            );
                        }
                        TypeSource::Parameter { name, span: source_span } => {
                            diag = diag.with_secondary_span(
                                span_to_source_span(source_span),
                                format!("parameter `{}` expects type `{}`", name, expected)
                            );
                        }
                        TypeSource::Return(source_span) => {
                            diag = diag.with_secondary_span(
                                span_to_source_span(source_span),
                                format!("function returns `{}`", expected)
                            );
                        }
                        TypeSource::Context { description, span: source_span } => {
                            diag = diag.with_secondary_span(
                                span_to_source_span(source_span),
                                format!("expected `{}` {}", expected, description)
                            );
                        }
                        TypeSource::Assignment(source_span) => {
                            diag = diag.with_secondary_span(
                                span_to_source_span(source_span),
                                format!("assignment target has type `{}`", expected)
                            );
                        }
                        TypeSource::BinaryOperator { op, side, span: source_span } => {
                            let side_str = match side {
                                BinaryOpSide::Left => "left",
                                BinaryOpSide::Right => "right",
                            };
                            diag = diag.with_secondary_span(
                                span_to_source_span(source_span),
                                format!("{} operand of `{}` expects `{}`", side_str, op, expected)
                            );
                        }
                        TypeSource::ConditionalBranch(source_span) => {
                            diag = diag.with_secondary_span(
                                span_to_source_span(source_span),
                                format!("all branches must have type `{}`", expected)
                            );
                        }
                        TypeSource::Unknown => {}
                    }
                }

                // Add helpful suggestions based on common mismatches
                if let Some(suggestion) = suggest_type_conversion(expected, found) {
                    diag = diag.with_suggestion(suggestion);
                }

                diag
            }

            TypeError::UndefinedVariable { name, span, similar_names } => {
                let mut diag = Diagnostic::error(
                    "E1001",
                    format!("undefined variable: `{}`", name)
                )
                .with_primary_span(
                    span_to_source_span(span),
                    "not found in this scope"
                );

                // Add similar name suggestions if available
                if let Some(similar) = similar_names {
                    if !similar.is_empty() {
                        let suggestion_text = if similar.len() == 1 {
                            format!("a similar name exists: `{}`", similar[0])
                        } else {
                            let names = similar.iter()
                                .map(|n| format!("`{}`", n))
                                .collect::<Vec<_>>()
                                .join(", ");
                            format!("similar names exist: {}", names)
                        };
                        diag = diag.with_child(
                            Diagnostic::help(suggestion_text)
                        );
                    }
                } else {
                    // No suggestions available - show generic note
                    diag = diag.with_child(
                        Diagnostic::note("variables must be declared before use")
                    );
                }

                diag
            }

            TypeError::UndefinedType(name, span) => {
                Diagnostic::error(
                    "E1002",
                    format!("undefined type: `{}`", name)
                )
                .with_primary_span(
                    span_to_source_span(span),
                    "type not found in this scope"
                )
                .with_child(
                    Diagnostic::note("types must be declared or imported before use")
                )
            }

            TypeError::CannotInfer(span) => {
                Diagnostic::error(
                    "E0002",
                    "cannot infer type"
                )
                .with_primary_span(
                    span_to_source_span(span),
                    "type cannot be inferred from context"
                )
                .with_child(
                    Diagnostic::help("try adding an explicit type annotation")
                )
            }

            TypeError::RecursiveType(span) => {
                Diagnostic::error(
                    "E0003",
                    "recursive type detected"
                )
                .with_primary_span(
                    span_to_source_span(span),
                    "this type definition is infinitely recursive"
                )
                .with_child(
                    Diagnostic::help("consider using indirection (e.g., Box, Rc, or Arc)")
                )
            }

            TypeError::WrongTypeArity { expected, found, span } => {
                Diagnostic::error(
                    "E0004",
                    "wrong number of type arguments"
                )
                .with_primary_span(
                    span_to_source_span(span),
                    format!("expected {} type argument(s), found {}", expected, found)
                )
            }

            // ============================================================================
            // Field Access Errors - Enhanced
            // ============================================================================

            TypeError::UndefinedField { type_name, field_name, span } => {
                Diagnostic::error(
                    "E1004",
                    format!("field not found: `{}`", field_name)
                )
                .with_primary_span(
                    span_to_source_span(span),
                    format!("type `{}` has no field `{}`", type_name, field_name)
                )
                .with_child(
                    Diagnostic::note("use dot notation to access struct fields")
                )
            }

            TypeError::FieldAccessOnNonStruct { type_name, span } => {
                Diagnostic::error(
                    "E1005",
                    "cannot access field on non-struct type"
                )
                .with_primary_span(
                    span_to_source_span(span),
                    format!("type `{}` is not a struct", type_name)
                )
            }

            // ============================================================================
            // Concurrency/Ownership Errors - Enhanced with helpful context
            // ============================================================================

            TypeError::NonTransferCapture { var_name, var_type, span } => {
                Diagnostic::error(
                    "E2001",
                    "cannot spawn task capturing non-Transfer value"
                )
                .with_primary_span(
                    span_to_source_span(span),
                    format!("variable `{}` of type `{}` does not implement Transfer", var_name, var_type)
                )
                .with_child(
                    Diagnostic::note(
                        "spawned tasks can only capture values that implement the Transfer trait"
                    )
                )
                .with_suggestion(
                    Suggestion::maybe_incorrect(
                        "consider using channels to communicate with the spawned task"
                    )
                )
            }

            TypeError::NonSharableShare { var_name, var_type, span } => {
                Diagnostic::error(
                    "E2002",
                    "cannot share non-Sharable value between tasks"
                )
                .with_primary_span(
                    span_to_source_span(span),
                    format!("variable `{}` of type `{}` does not implement Sharable", var_name, var_type)
                )
                .with_child(
                    Diagnostic::note(
                        "values shared between tasks must implement the Sharable trait"
                    )
                )
            }

            TypeError::MutableCaptureOfImmutable { var_name, span } => {
                Diagnostic::error(
                    "E2003",
                    "cannot mutably capture immutable variable"
                )
                .with_primary_span(
                    span_to_source_span(span),
                    format!("variable `{}` is immutable", var_name)
                )
                .with_child(
                    Diagnostic::help(
                        format!("consider declaring `{}` as mutable: `mut {}`", var_name, var_name)
                    )
                )
            }

            TypeError::MutableCaptureInSpawn { var_name, span } => {
                Diagnostic::error(
                    "E2004",
                    "cannot mutably capture in spawned task"
                )
                .with_primary_span(
                    span_to_source_span(span),
                    format!("spawned closures cannot hold mutable borrows of `{}`", var_name)
                )
                .with_child(
                    Diagnostic::note(
                        "spawned tasks may outlive the parent scope, making mutable borrows unsafe"
                    )
                )
            }

            // ============================================================================
            // Trait/Generic Errors - Enhanced
            // ============================================================================

            TypeError::TraitNotImplemented { ty, trait_name, span } => {
                Diagnostic::error(
                    "E0005",
                    "trait bound not satisfied"
                )
                .with_primary_span(
                    span_to_source_span(span),
                    format!("type `{}` does not implement trait `{}`", ty, trait_name)
                )
                .with_child(
                    Diagnostic::note(
                        format!("required because of trait bound `{}: {}`", ty, trait_name)
                    )
                )
            }

            TypeError::BoundNotSatisfied { type_arg, param, bound, span } => {
                Diagnostic::error(
                    "E0006",
                    "type parameter bound not satisfied"
                )
                .with_primary_span(
                    span_to_source_span(span),
                    format!("type `{}` does not satisfy bound `{}: {}`", type_arg, param, bound)
                )
            }

            // ============================================================================
            // Pattern Matching Errors
            // ============================================================================

            TypeError::NonExhaustivePatterns { missing, span } => {
                Diagnostic::error(
                    "E4001",
                    "non-exhaustive patterns"
                )
                .with_primary_span(
                    span_to_source_span(span),
                    format!("missing patterns: {}", missing)
                )
                .with_child(
                    Diagnostic::help("ensure all possible cases are covered")
                )
            }

            TypeError::UnreachablePattern { span } => {
                Diagnostic::error(
                    "E4002",
                    "unreachable pattern"
                )
                .with_primary_span(
                    span_to_source_span(span),
                    "this pattern will never match"
                )
                .with_child(
                    Diagnostic::note("previous patterns already cover this case")
                )
            }

            // ============================================================================
            // Return Type Errors
            // ============================================================================

            TypeError::ReturnTypeMismatch { expected, found, span } => {
                Diagnostic::error(
                    "E0007",
                    "return type mismatch"
                )
                .with_primary_span(
                    span_to_source_span(span),
                    format!("expected return type `{}`, found `{}`", expected, found)
                )
            }

            // ============================================================================
            // Iteration Errors
            // ============================================================================

            TypeError::NotIterable { found, span } => {
                Diagnostic::error(
                    "E0008",
                    "type is not iterable"
                )
                .with_primary_span(
                    span_to_source_span(span),
                    format!("type `{}` cannot be iterated over", found)
                )
                .with_child(
                    Diagnostic::note("only types implementing the Iterator trait can be used in for loops")
                )
            }

            // ============================================================================
            // Default fallback for other errors
            // ============================================================================

            _ => {
                // For errors without specific enhanced diagnostics, create a basic diagnostic
                Diagnostic::error(
                    "E9999",
                    format!("{}", self)
                )
            }
        }
    }
}

/// Helper function to convert Aria Span to DiagnosticSourceSpan
fn span_to_source_span(span: &Span) -> SourceSpan {
    // For now, create a simple span without file information
    // TODO: Thread source file information through the type checker
    SourceSpan::new(
        "unknown",  // Placeholder - should be replaced with actual file path
        span.start,
        span.end
    )
}

/// Suggest type conversions for common type mismatches
fn suggest_type_conversion(expected: &str, found: &str) -> Option<Suggestion> {
    // String <-> Int conversions
    if expected == "String" && found == "Int" {
        return Some(
            Suggestion::machine_applicable(
                "convert to string using `.to_string()`"
            )
        );
    }

    if expected == "Int" && found == "String" {
        return Some(
            Suggestion::maybe_incorrect(
                "parse string to int using `.parse()?` or `.parse().unwrap()`"
            )
        );
    }

    // Float <-> Int conversions
    if expected == "Float" && found == "Int" {
        return Some(
            Suggestion::machine_applicable(
                "convert to float using `as Float`"
            )
        );
    }

    if expected == "Int" && found == "Float" {
        return Some(
            Suggestion::maybe_incorrect(
                "convert to int using `.floor()`, `.ceil()`, or `.round()` and then `as Int`"
            )
        );
    }

    // Optional wrapping
    if expected.ends_with('?') && !found.ends_with('?') {
        let base_expected = expected.trim_end_matches('?');
        if base_expected == found {
            return Some(
                Suggestion::machine_applicable(
                    "wrap value in Some: `Some(value)`"
                )
            );
        }
    }

    // Array element type mismatch
    if expected.starts_with('[') && found.starts_with('[') {
        return Some(
            Suggestion::maybe_incorrect(
                "ensure all array elements have the same type"
            )
        );
    }

    None
}

/// Calculate Levenshtein distance between two strings.
///
/// This is used for typo suggestions in error messages.
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let len_a = a.chars().count();
    let len_b = b.chars().count();

    if len_a == 0 {
        return len_b;
    }
    if len_b == 0 {
        return len_a;
    }

    let mut matrix = vec![vec![0; len_b + 1]; len_a + 1];

    // Initialize first column and row
    for i in 0..=len_a {
        matrix[i][0] = i;
    }
    for j in 0..=len_b {
        matrix[0][j] = j;
    }

    let chars_a: Vec<char> = a.chars().collect();
    let chars_b: Vec<char> = b.chars().collect();

    // Calculate distances
    for i in 1..=len_a {
        for j in 1..=len_b {
            let cost = if chars_a[i - 1] == chars_b[j - 1] { 0 } else { 1 };
            matrix[i][j] = std::cmp::min(
                std::cmp::min(
                    matrix[i - 1][j] + 1,      // deletion
                    matrix[i][j - 1] + 1       // insertion
                ),
                matrix[i - 1][j - 1] + cost    // substitution
            );
        }
    }

    matrix[len_a][len_b]
}

/// Find similar names using Levenshtein distance.
///
/// Returns names within a distance threshold, sorted by distance.
fn find_similar_names(target: &str, candidates: &[String], max_distance: usize) -> Vec<String> {
    let mut matches: Vec<(String, usize)> = candidates
        .iter()
        .map(|name| (name.clone(), levenshtein_distance(target, name)))
        .filter(|(_, dist)| *dist <= max_distance)
        .collect();

    // Sort by distance (closest first)
    matches.sort_by_key(|(_, dist)| *dist);

    // Return just the names
    matches.into_iter().map(|(name, _)| name).collect()
}
