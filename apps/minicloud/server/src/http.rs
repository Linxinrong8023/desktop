use axum::{
    Json, Router,
    extract::{Path, State, rejection::JsonRejection},
    http::StatusCode,
    routing::get,
};
use ora_contracts::minicloud::*;
use ora_controller::{CloneOperation, ControllerHandle, Error};
use ora_node_protocol::*;

#[derive(Clone)]
struct App {
    controller: ControllerHandle,
    node: NodeId,
}
type Failure = (StatusCode, Json<MiniError>);

/// Composes only the clone application surface; no Desktop bindings or Node wire messages leak through HTTP.
pub(super) fn router(controller: ControllerHandle, node: NodeId) -> Router {
    Router::new()
        .route("/api/clones", get(list).post(submit))
        .route("/api/clones/{execution}", get(detail))
        .with_state(App { controller, node })
}

/// Maps adapter errors without turning a failed HTTP call into a failed clone result.
fn failure(error: Error) -> Failure {
    let (status, code) = match error {
        Error::Conflict | Error::AlreadyRunning => (StatusCode::CONFLICT, MiniErrorCode::Conflict),
        Error::Validation(_) => (StatusCode::BAD_REQUEST, MiniErrorCode::InvalidInput),
        Error::Io(_)
        | Error::Sql(_)
        | Error::Encoding(_)
        | Error::InvalidStorage
        | Error::Injected => (StatusCode::SERVICE_UNAVAILABLE, MiniErrorCode::Unavailable),
    };
    (status, Json(MiniError { code }))
}

/// Returns acceptance only after Controller commits the original request identity and full intent.
async fn submit(
    State(app): State<App>,
    input: Result<Json<MiniCloneRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<MiniCloneAccepted>), Failure> {
    let Json(input) = input.map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(MiniError {
                code: MiniErrorCode::InvalidInput,
            }),
        )
    })?;
    let repository = CloneRepositoryUrl::parse(&input.repository).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(MiniError {
                code: MiniErrorCode::InvalidInput,
            }),
        )
    })?;
    let command = app
        .controller
        .accept_clone(
            RequestId::new(input.request_id.clone()),
            CloneExecutionSpec {
                node_id: app.node,
                repository,
                branch: BranchName::new(input.branch),
            },
        )
        .await
        .map_err(failure)?;
    Ok((
        StatusCode::ACCEPTED,
        Json(MiniCloneAccepted {
            request_id: input.request_id,
            operation_id: command.operation_id.as_str().into(),
            execution_id: command.execution_id.as_str().into(),
        }),
    ))
}

/// Lists durable intent regardless of current Node connectivity.
async fn list(State(app): State<App>) -> Result<Json<Vec<MiniCloneOperation>>, Failure> {
    Ok(Json(
        app.controller
            .operations()
            .await
            .map_err(failure)?
            .into_iter()
            .map(present)
            .collect(),
    ))
}

/// An absent execution is not the same as a pending result.
async fn detail(
    State(app): State<App>,
    Path(execution): Path<String>,
) -> Result<Json<MiniCloneOperation>, Failure> {
    app.controller
        .operation(ExecutionId::new(execution))
        .await
        .map_err(failure)?
        .map(present)
        .map(Json)
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(MiniError {
                code: MiniErrorCode::NotFound,
            }),
        ))
}

/// Projects immutable Controller facts; pending deliberately makes no live-progress claim.
fn present(operation: CloneOperation) -> MiniCloneOperation {
    let state = match operation.result {
        None => MiniCloneState::Pending,
        Some(CloneExecutionResult::CloneReady(result)) => MiniCloneState::Succeeded {
            path: result.path.as_str().into(),
            commit: result.commit.as_str().into(),
        },
        Some(CloneExecutionResult::CloneFailed(result)) => MiniCloneState::Failed {
            reason: match result.failure {
                CloneFailureCode::SourceUnavailable => MiniCloneFailure::SourceUnavailable,
                CloneFailureCode::BranchNotFound => MiniCloneFailure::BranchNotFound,
                CloneFailureCode::DestinationConflict => MiniCloneFailure::DestinationConflict,
                CloneFailureCode::OperationFailed => MiniCloneFailure::OperationFailed,
            },
            retained_path: match result.residual {
                CloneResidual::NoDirectory {} => None,
                CloneResidual::Retained { path, .. } => Some(path.as_str().into()),
            },
        },
    };
    let command = operation.command;
    MiniCloneOperation {
        operation_id: command.operation_id.as_str().into(),
        execution_id: command.execution_id.as_str().into(),
        node_id: command.payload.spec.node_id.as_str().into(),
        repository: command.payload.spec.repository.as_str().into(),
        branch: command.payload.spec.branch.as_str().into(),
        state,
    }
}
