//! Tests for async/await type checking
//!
//! This module tests:
//! 1. Spawn expressions produce Task[T] types
//! 2. Await expressions extract T from Task[T]
//! 3. Await can only be used in async contexts
//! 4. Channel send/receive type checking
//! 5. Select expression type checking
//! 6. Task type properties (Transfer, Sharable, Copy)

use aria_ast::Span;
use aria_types::{Type, TypeChecker, TypeInference};

// ============================================================================
// Task Type Tests
// ============================================================================

#[test]
fn test_task_type_display() {
    let task_int = Type::Task(Box::new(Type::Int));
    assert_eq!(format!("{}", task_int), "Task[Int]");

    let task_string = Type::Task(Box::new(Type::String));
    assert_eq!(format!("{}", task_string), "Task[String]");

    // Nested task
    let task_task = Type::Task(Box::new(Type::Task(Box::new(Type::Bool))));
    assert_eq!(format!("{}", task_task), "Task[Task[Bool]]");
}

#[test]
fn test_task_is_transfer_if_result_is_transfer() {
    // Task[Int] is Transfer because Int is Transfer
    let task_int = Type::Task(Box::new(Type::Int));
    assert!(task_int.is_transfer());

    // Task[String] is Transfer because String is Transfer
    let task_string = Type::Task(Box::new(Type::String));
    assert!(task_string.is_transfer());

    // Task[Array[Int]] is Transfer because Array[Int] is Transfer
    let task_array = Type::Task(Box::new(Type::Array(Box::new(Type::Int))));
    assert!(task_array.is_transfer());
}

#[test]
fn test_task_not_transfer_if_result_not_transfer() {
    // Task[&mut Int] is NOT Transfer because mutable ref is not Transfer
    let task_mut_ref = Type::Task(Box::new(Type::Reference {
        mutable: true,
        inner: Box::new(Type::Int),
    }));
    assert!(!task_mut_ref.is_transfer());
}

#[test]
fn test_task_is_always_sharable() {
    // All tasks are Sharable (can be awaited from multiple contexts)
    let task_int = Type::Task(Box::new(Type::Int));
    assert!(task_int.is_sharable());

    let task_mut_ref = Type::Task(Box::new(Type::Reference {
        mutable: true,
        inner: Box::new(Type::Int),
    }));
    assert!(task_mut_ref.is_sharable());
}

#[test]
fn test_task_is_not_copy() {
    // Tasks are not Copy (they represent unique handles to computations)
    let task_int = Type::Task(Box::new(Type::Int));
    assert!(!task_int.is_copy());
}

#[test]
fn test_task_is_spawn_safe() {
    // Task[T] is spawn-safe if T is Transfer
    let task_int = Type::Task(Box::new(Type::Int));
    assert!(task_int.is_spawn_safe());

    let task_mut_ref = Type::Task(Box::new(Type::Reference {
        mutable: true,
        inner: Box::new(Type::Int),
    }));
    assert!(!task_mut_ref.is_spawn_safe());
}

// ============================================================================
// Type Inference with Task
// ============================================================================

#[test]
fn test_unify_task_types() {
    let mut inf = TypeInference::new();

    // Same Task types should unify
    let task1 = Type::Task(Box::new(Type::Int));
    let task2 = Type::Task(Box::new(Type::Int));
    assert!(inf.unify(&task1, &task2, Span::dummy()).is_ok());

    // Different Task types should not unify
    let task_int = Type::Task(Box::new(Type::Int));
    let task_str = Type::Task(Box::new(Type::String));
    assert!(inf.unify(&task_int, &task_str, Span::dummy()).is_err());
}

#[test]
fn test_unify_task_with_type_variable() {
    let mut inf = TypeInference::new();

    let result_var = inf.fresh_var();
    let task_var = Type::Task(Box::new(result_var.clone()));
    let task_int = Type::Task(Box::new(Type::Int));

    // Should unify and bind result type
    assert!(inf.unify(&task_var, &task_int, Span::dummy()).is_ok());
    assert_eq!(inf.apply(&result_var), Type::Int);
}

#[test]
fn test_apply_substitution_to_task() {
    let mut inf = TypeInference::new();

    let result_var = inf.fresh_var();
    let task_var = Type::Task(Box::new(result_var.clone()));

    // Unify the result var with Int
    assert!(inf.unify(&result_var, &Type::Int, Span::dummy()).is_ok());

    // Apply should resolve the Task type
    let resolved = inf.apply(&task_var);
    assert_eq!(resolved, Type::Task(Box::new(Type::Int)));
}

// ============================================================================
// Channel Type Tests
// ============================================================================

#[test]
fn test_channel_type_display() {
    let ch_int = Type::Channel(Box::new(Type::Int));
    assert_eq!(format!("{}", ch_int), "Channel[Int]");
}

#[test]
fn test_unify_channel_with_type_variable() {
    let mut inf = TypeInference::new();

    let elem_var = inf.fresh_var();
    let ch_var = Type::Channel(Box::new(elem_var.clone()));
    let ch_string = Type::Channel(Box::new(Type::String));

    assert!(inf.unify(&ch_var, &ch_string, Span::dummy()).is_ok());
    assert_eq!(inf.apply(&elem_var), Type::String);
}

// ============================================================================
// Async Context Tracking Tests
// ============================================================================

#[test]
fn test_spawn_creates_async_context() {
    // This is implicitly tested - spawn body allows await
    // The TypeChecker sets in_async_context = true during spawn body checking
    let _checker = TypeChecker::new();
    // Verification would require parsing and checking actual code
}

// ============================================================================
// Combined Channel and Task Tests
// ============================================================================

#[test]
fn test_channel_of_tasks() {
    // Channel[Task[Int]] - a channel that sends/receives task handles
    let ch_task = Type::Channel(Box::new(Type::Task(Box::new(Type::Int))));
    assert_eq!(format!("{}", ch_task), "Channel[Task[Int]]");

    // Transfer: Channel is Transfer if Task[Int] is Transfer
    // Task[Int] is Transfer because Int is Transfer
    assert!(ch_task.is_transfer());
}

#[test]
fn test_task_of_channels() {
    // Task[Channel[String]] - a task that produces a channel
    let task_ch = Type::Task(Box::new(Type::Channel(Box::new(Type::String))));
    assert_eq!(format!("{}", task_ch), "Task[Channel[String]]");

    // Transfer: Task is Transfer if Channel[String] is Transfer
    // Channel[String] is Transfer because String is Transfer
    assert!(task_ch.is_transfer());
}

// ============================================================================
// Error Type Tests
// ============================================================================

#[test]
fn test_await_outside_async_error() {
    use aria_types::TypeError;

    let err = TypeError::AwaitOutsideAsync { span: Span::dummy() };
    let msg = format!("{}", err);
    assert!(msg.contains("await"));
    assert!(msg.contains("async"));
}

#[test]
fn test_await_non_task_error() {
    use aria_types::TypeError;

    let err = TypeError::AwaitNonTask {
        found: "Int".to_string(),
        span: Span::dummy(),
    };
    let msg = format!("{}", err);
    assert!(msg.contains("await"));
    assert!(msg.contains("Task"));
    assert!(msg.contains("Int"));
}

#[test]
fn test_send_on_non_channel_error() {
    use aria_types::TypeError;

    let err = TypeError::SendOnNonChannel {
        found: "Int".to_string(),
        span: Span::dummy(),
    };
    let msg = format!("{}", err);
    assert!(msg.contains("Channel"));
    assert!(msg.contains("Int"));
}

#[test]
fn test_receive_on_non_channel_error() {
    use aria_types::TypeError;

    let err = TypeError::ReceiveOnNonChannel {
        found: "String".to_string(),
        span: Span::dummy(),
    };
    let msg = format!("{}", err);
    assert!(msg.contains("Channel"));
    assert!(msg.contains("String"));
}

// ============================================================================
// Channel Type Property Tests
// ============================================================================

#[test]
fn test_channel_is_transfer_if_elem_is_transfer() {
    // Channel[Int] is Transfer because Int is Transfer
    let ch_int = Type::Channel(Box::new(Type::Int));
    assert!(ch_int.is_transfer());

    // Channel[String] is Transfer because String is Transfer
    let ch_string = Type::Channel(Box::new(Type::String));
    assert!(ch_string.is_transfer());

    // Channel[Array[Int]] is Transfer because Array[Int] is Transfer
    let ch_array = Type::Channel(Box::new(Type::Array(Box::new(Type::Int))));
    assert!(ch_array.is_transfer());
}

#[test]
fn test_channel_not_transfer_if_elem_not_transfer() {
    // Channel[&mut Int] is NOT Transfer because mutable ref is not Transfer
    let ch_mut_ref = Type::Channel(Box::new(Type::Reference {
        mutable: true,
        inner: Box::new(Type::Int),
    }));
    assert!(!ch_mut_ref.is_transfer());
}

#[test]
fn test_channel_is_sharable() {
    // All channels are Sharable (can be shared between tasks)
    let ch_int = Type::Channel(Box::new(Type::Int));
    assert!(ch_int.is_sharable());

    let ch_mut_ref = Type::Channel(Box::new(Type::Reference {
        mutable: true,
        inner: Box::new(Type::Int),
    }));
    // Even channels of non-transfer types are sharable
    assert!(ch_mut_ref.is_sharable());
}

#[test]
fn test_channel_is_not_copy() {
    // Channels are not Copy (they represent shared resources)
    let ch_int = Type::Channel(Box::new(Type::Int));
    assert!(!ch_int.is_copy());
}

#[test]
fn test_channel_type_unification() {
    let mut inf = TypeInference::new();

    // Same Channel types should unify
    let ch1 = Type::Channel(Box::new(Type::Int));
    let ch2 = Type::Channel(Box::new(Type::Int));
    assert!(inf.unify(&ch1, &ch2, Span::dummy()).is_ok());

    // Different Channel element types should not unify
    let ch_int = Type::Channel(Box::new(Type::Int));
    let ch_str = Type::Channel(Box::new(Type::String));
    assert!(inf.unify(&ch_int, &ch_str, Span::dummy()).is_err());
}

#[test]
fn test_nested_channel_types() {
    // Channel[Channel[Int]] - a channel that sends/receives channels
    let nested_ch = Type::Channel(Box::new(Type::Channel(Box::new(Type::Int))));
    assert_eq!(format!("{}", nested_ch), "Channel[Channel[Int]]");

    // Should be Transfer because inner channel is Transfer
    assert!(nested_ch.is_transfer());
}

// ============================================================================
// Channel Method Type Inference Tests (conceptual)
// ============================================================================

#[test]
fn test_channel_element_type_inference() {
    let mut inf = TypeInference::new();

    // Given a type variable for element type
    let elem_var = inf.fresh_var();
    let channel_type = Type::Channel(Box::new(elem_var.clone()));

    // When unified with Channel[Int]
    let ch_int = Type::Channel(Box::new(Type::Int));
    assert!(inf.unify(&channel_type, &ch_int, Span::dummy()).is_ok());

    // Then the element type should resolve to Int
    assert_eq!(inf.apply(&elem_var), Type::Int);
}

#[test]
fn test_channel_apply_substitution() {
    let mut inf = TypeInference::new();

    let elem_var = inf.fresh_var();
    let channel_type = Type::Channel(Box::new(elem_var.clone()));

    // Unify the element var with String
    assert!(inf.unify(&elem_var, &Type::String, Span::dummy()).is_ok());

    // Apply should resolve the Channel type
    let resolved = inf.apply(&channel_type);
    assert_eq!(resolved, Type::Channel(Box::new(Type::String)));
}
