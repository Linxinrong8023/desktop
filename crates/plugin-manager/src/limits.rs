use ora_utils::archive::ExtractLimits;

/// Cap on the `.orax` archive accepted before extraction begins.
const MAX_PACKAGE_ARCHIVE_BYTES: u64 = 512 * 1024 * 1024;

/// Cap on the cumulative bytes one package may materialize on disk.
const MAX_PACKAGE_TOTAL_BYTES: u64 = 1024 * 1024 * 1024;

/// Returns the extraction limits applied to every plugin package.
///
/// A plugin package is not a document bundle. An Agent Plugin may ship the CLI it drives so the
/// user does not have to install one, and those are single native binaries in the hundreds of
/// megabytes — OpenCode is roughly 176 MiB unpacked from a 58 MiB archive. `ExtractLimits`'s
/// generic default is sized for text packages and would refuse such a release before extraction
/// ever started, so plugin installs declare their own budget here rather than inheriting one
/// tuned for a different kind of payload.
///
/// The entry-count and path limits stay at the shared defaults: shipping a large binary is a
/// reason to raise the byte budgets, not to accept deeper trees or more files.
pub(crate) fn package_extract_limits() -> ExtractLimits {
    ExtractLimits {
        max_archive_bytes: MAX_PACKAGE_ARCHIVE_BYTES,
        max_total_bytes: MAX_PACKAGE_TOTAL_BYTES,
        ..ExtractLimits::default()
    }
}
