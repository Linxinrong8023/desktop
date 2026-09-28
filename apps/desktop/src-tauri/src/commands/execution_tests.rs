//! Pins executor wiring independently of domain-specific failure and rollback triggers.

use super::{run_async_backend, run_async_backend_with_request_id, run_backend};
use ora_backend::BackendError;
use ora_contracts::{ContractError, EmptyErrorParams, PublicError, RequestId};
use ora_logging::with_recorded_trace_logging;
use pretty_assertions::assert_eq;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::registry::LookupSpan;

#[derive(Clone, Copy)]
enum Executor {
    Blocking,
    Async,
    AsyncWithRequestId,
}

#[derive(Clone, Copy)]
enum Outcome {
    Success,
    Failure,
    RollbackFailure,
    Panic,
}

/// Captures both explicit event fields and the actual enclosing request span.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Event {
    pub(super) fields: Fields,
    pub(super) scope: Vec<Fields>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Fields(pub(super) BTreeMap<String, String>);

impl Visit for Fields {
    /// Keeps string values unquoted for comparison with the serialized response.
    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.insert(field.name().into(), value.into());
    }

    /// Captures display-formatted IDs and numeric completion fields as well.
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.0.insert(field.name().into(), format!("{value:?}"));
    }
}

#[derive(Clone, Default)]
pub(super) struct Recorder(pub(super) Arc<Mutex<Vec<Event>>>);

impl<S> Layer<S> for Recorder
where
    S: tracing::Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    /// Stores initial span fields, including the logical Tauri span name.
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let mut fields = Fields::default();
        attrs.record(&mut fields);
        ctx.span(id).unwrap().extensions_mut().insert(fields);
    }

    /// Request IDs are recorded after span creation by the production logging helper.
    fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
        let span = ctx.span(id).unwrap();
        let mut extensions = span.extensions_mut();
        values.record(extensions.get_mut::<Fields>().unwrap());
    }

    /// Reads scope from the subscriber rather than inferring correlation from event fields.
    fn on_event(&self, event: &tracing::Event<'_>, ctx: Context<'_, S>) {
        let mut fields = Fields::default();
        event.record(&mut fields);
        let scope = ctx
            .event_scope(event)
            .into_iter()
            .flat_map(|scope| scope.from_root())
            .map(|span| span.extensions().get::<Fields>().unwrap().clone())
            .collect();
        self.0.lock().unwrap().push(Event { fields, scope });
    }
}

/// Emits representative domain diagnostics without reproducing domain rollback policies.
fn operation(context: &str, request: &str, outcome: Outcome) -> Result<String, BackendError> {
    ora_logging::ora_info!("executor operation entered");
    match outcome {
        Outcome::Success => Ok(format!("{context}:{request}")),
        Outcome::Failure => Err(BackendError::invalid_proxy_settings(
            "test operation failed",
        )),
        Outcome::RollbackFailure => {
            ora_logging::ora_error!("test rollback failed");
            Err(BackendError::invalid_proxy_settings(
                "test operation failed",
            ))
        }
        Outcome::Panic => panic!("executor panic probe"),
    }
}

/// Exercises the real executor and asserts the entire completion sequence after it returns.
fn verify(executor: Executor, outcome: Outcome) {
    let recorder = Recorder::default();
    let result = with_recorded_trace_logging(recorder.clone(), || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        runtime.block_on(async {
            match executor {
                Executor::Blocking => {
                    // A scoped subscriber is thread-local. Transport it to the worker without
                    // creating or entering any span: span propagation remains the executor's job.
                    let dispatch = tracing::dispatcher::get_default(Clone::clone);
                    run_backend(
                        "executor_test",
                        "context",
                        "request",
                        move |context, request| {
                            tracing::dispatcher::with_default(&dispatch, || {
                                operation(context, request, outcome)
                            })
                        },
                    )
                    .await
                }
                Executor::AsyncWithRequestId => {
                    run_async_backend_with_request_id("executor_test", |request_id| {
                        Box::pin(async move {
                            tokio::task::yield_now().await;
                            ora_logging::ora_info!(request_id = %request_id, "correlation received");
                            operation("context", "request", outcome)
                        })
                    })
                    .await
                }
                Executor::Async => {
                    run_async_backend(
                        "executor_test",
                        Box::pin(async {
                            // Force a second poll to exercise instrumentation across suspension.
                            tokio::task::yield_now().await;
                            operation("context", "request", outcome)
                        }),
                    )
                    .await
                }
            }
        })
    });
    let events = recorder.0.lock().unwrap().clone();
    let entered = events
        .iter()
        .find(|event| {
            event.fields.0.get("message").map(String::as_str) == Some("executor operation entered")
        })
        .unwrap();
    assert_eq!(entered.scope.len(), 1);
    let request_span = entered.scope[0].clone();
    assert_eq!(
        request_span.0.get("span").map(String::as_str),
        Some("tauri_command")
    );
    let request_id = request_span.0.get("request_id").unwrap();
    if let Executor::AsyncWithRequestId = executor {
        let correlation = events
            .iter()
            .find(|event| {
                event.fields.0.get("message").map(String::as_str) == Some("correlation received")
            })
            .unwrap();
        assert_eq!(correlation.fields.0.get("request_id"), Some(request_id));
        assert_eq!(correlation.scope, vec![request_span.clone()]);
    }
    let parsed_id: RequestId = serde_json::from_value(serde_json::json!(request_id)).unwrap();
    let expected_outcome = match outcome {
        Outcome::Success => {
            assert_eq!(result.unwrap(), "context:request");
            "success"
        }
        Outcome::Failure | Outcome::RollbackFailure | Outcome::Panic => {
            let public_error = match outcome {
                Outcome::Panic => PublicError::InternalError(EmptyErrorParams {}),
                Outcome::Failure | Outcome::RollbackFailure => {
                    PublicError::InvalidRequest(EmptyErrorParams {})
                }
                Outcome::Success => unreachable!(),
            };
            assert_eq!(
                serde_json::to_value(result.unwrap_err()).unwrap(),
                serde_json::to_value(ContractError {
                    error: public_error,
                    request_id: parsed_id
                })
                .unwrap()
            );
            "failure"
        }
    };
    let completions: Vec<_> = events
        .iter()
        .filter(|event| {
            event.fields.0.get("message").map(String::as_str) == Some("request completed")
        })
        .collect();
    assert_eq!(
        completions.len(),
        1,
        "all completion outcomes, including abandoned, count"
    );
    let completion = completions[0];
    assert_eq!(completion.scope, vec![request_span.clone()]);
    assert_eq!(
        ["operation", "outcome", "request_id"].map(|key| completion
            .fields
            .0
            .get(key)
            .map(String::as_str)),
        [
            Some("executor_test"),
            Some(expected_outcome),
            Some(request_id.as_str())
        ]
    );
    assert!(completion.fields.0["duration_ms"].parse::<u64>().is_ok());
    let diagnostics: Vec<_> = events
        .iter()
        .filter(|event| {
            event.fields.0.get("message").map(String::as_str) == Some("test rollback failed")
        })
        .map(|event| event.scope.clone())
        .collect();
    assert_eq!(
        diagnostics,
        match outcome {
            Outcome::RollbackFailure => vec![vec![request_span]],
            Outcome::Success | Outcome::Failure | Outcome::Panic => vec![],
        }
    );
    if let Outcome::Panic = outcome {
        assert_eq!(
            completion.fields.0["error.message"],
            "Desktop command execution failed"
        );
        assert!(completion.fields.0["error.chain"].contains("executor panic probe"));
    }
}

/// Success preserves the blocking operation's output and completes once.
#[test]
fn blocking_success() {
    verify(Executor::Blocking, Outcome::Success);
}

/// Backend errors retain the blocking request's correlation and complete once.
#[test]
fn blocking_failure() {
    verify(Executor::Blocking, Outcome::Failure);
}

/// Additional blocking diagnostics do not complete a second request.
#[test]
fn blocking_rollback_diagnostic() {
    verify(Executor::Blocking, Outcome::RollbackFailure);
}

/// A worker panic becomes a correlated internal error instead of escaping the executor.
#[test]
fn blocking_panic() {
    verify(Executor::Blocking, Outcome::Panic);
}

/// Async success retains instrumentation across a pending poll and completes once.
#[test]
fn async_success() {
    verify(Executor::Async, Outcome::Success);
}

/// Async errors retain the request's correlation and complete once.
#[test]
fn async_failure() {
    verify(Executor::Async, Outcome::Failure);
}

/// Additional async diagnostics share the request span without a second completion.
#[test]
fn async_rollback_diagnostic() {
    verify(Executor::Async, Outcome::RollbackFailure);
}

/// Diagnostic correlation preserves the successful result and the sole completion.
#[test]
fn async_with_request_id_success() {
    verify(Executor::AsyncWithRequestId, Outcome::Success);
}

/// Diagnostic correlation matches both the failure response and its completion.
#[test]
fn async_with_request_id_failure() {
    verify(Executor::AsyncWithRequestId, Outcome::Failure);
}

/// A correlated secondary diagnostic never adds another request completion.
#[test]
fn async_with_request_id_rollback_diagnostic() {
    verify(Executor::AsyncWithRequestId, Outcome::RollbackFailure);
}
