//! Marketplace source failures and their secret-free public error mapping.

use super::artifact_retrieval::ArtifactRetrievalError;
use crate::error::{BackendError, ErrorClassification};
use ora_contracts::{
    EmptyErrorParams, MarketplaceArtifactRetrievalFieldInvalidParams, PublicError,
};
use ora_domain::PluginIdError;
use ora_plugin_registry::RegistryError;
use thiserror::Error;

/// Reports failures while loading, validating, or persisting the marketplace source list.
#[derive(Debug, Error)]
pub(crate) enum MarketplaceSourceStoreError {
    #[error("invalid marketplace source: {0}")]
    Validation(#[from] RegistryError),
    #[error("marketplace source already exists: {0}")]
    Duplicate(String),
    #[error("marketplace source was not found: {0}")]
    NotFound(String),
    #[error("marketplace source repository operation failed: {0}")]
    Repository(#[from] ora_db::DatabaseError),
    #[error("invalid marketplace artifact retrieval configuration: {0}")]
    ArtifactRetrieval(#[from] ArtifactRetrievalError),
    /// A persisted binding holds a namespace this version cannot represent, so the source cannot
    /// be used without either inventing a new identity for it or silently changing an existing
    /// one — both of which would detach its already-installed plugins.
    #[error("persisted marketplace source namespace is unusable: {0}")]
    CorruptNamespace(#[from] PluginIdError),
}

/// Preserves actionable validation details while keeping stored corruption an internal failure.
pub(crate) fn map_marketplace_source_error(error: MarketplaceSourceStoreError) -> BackendError {
    match error {
        MarketplaceSourceStoreError::Validation(error) => BackendError::new(
            ErrorClassification::InvalidRequest,
            PublicError::InvalidRequest(EmptyErrorParams {}),
            format!("invalid plugin marketplace source: {error}"),
        ),
        MarketplaceSourceStoreError::Duplicate(url) => BackendError::new(
            ErrorClassification::InvalidRequest,
            PublicError::InvalidRequest(EmptyErrorParams {}),
            format!("plugin marketplace source already exists: {url}"),
        ),
        MarketplaceSourceStoreError::NotFound(url) => BackendError::new(
            ErrorClassification::NotFound,
            PublicError::InvalidRequest(EmptyErrorParams {}),
            format!("plugin marketplace source was not found: {url}"),
        ),
        MarketplaceSourceStoreError::ArtifactRetrieval(error) => match error {
            ArtifactRetrievalError::CredentialsRequired => BackendError::new(
                ErrorClassification::InvalidRequest,
                PublicError::MarketplaceS3CredentialsRequired(EmptyErrorParams {}),
                error.to_string(),
            ),
            ArtifactRetrievalError::InvalidField(field) => BackendError::new(
                ErrorClassification::InvalidRequest,
                PublicError::MarketplaceArtifactRetrievalFieldInvalid(
                    MarketplaceArtifactRetrievalFieldInvalidParams {
                        field: field.to_owned(),
                    },
                ),
                error.to_string(),
            ),
            ArtifactRetrievalError::InvalidPersistedConfiguration => BackendError::internal(
                "failed to load configured plugin marketplace artifact retrieval",
                error,
            ),
        },
        error @ (MarketplaceSourceStoreError::Repository(_)
        | MarketplaceSourceStoreError::CorruptNamespace(_)) => BackendError::internal(
            "failed to persist configured plugin marketplace sources",
            error,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    /// Clients can distinguish credential requirements and every invalid field without values.
    #[test]
    fn maps_artifact_retrieval_validation_to_actionable_public_errors() {
        let mut cases = vec![(
            ArtifactRetrievalError::CredentialsRequired,
            PublicError::MarketplaceS3CredentialsRequired(EmptyErrorParams {}),
        )];
        for field in [
            "endpoint",
            "bucket",
            "region",
            "access key id",
            "secret access key",
        ] {
            cases.push((
                ArtifactRetrievalError::InvalidField(field),
                PublicError::MarketplaceArtifactRetrievalFieldInvalid(
                    MarketplaceArtifactRetrievalFieldInvalidParams {
                        field: field.to_owned(),
                    },
                ),
            ));
        }
        for (failure, public_error) in cases {
            let error = map_marketplace_source_error(failure.into());
            assert_eq!(
                (error.classification(), error.public_error()),
                (ErrorClassification::InvalidRequest, &public_error),
            );
        }
    }

    /// Bad stored configuration is not misreported as a malformed client request.
    #[test]
    fn maps_corrupt_artifact_retrieval_to_an_internal_error() {
        let error = map_marketplace_source_error(
            ArtifactRetrievalError::InvalidPersistedConfiguration.into(),
        );
        assert_eq!(
            (error.classification(), error.public_error()),
            (
                ErrorClassification::Internal,
                &PublicError::InternalError(EmptyErrorParams {}),
            ),
        );
    }
}
