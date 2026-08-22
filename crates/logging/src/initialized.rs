use crate::{LogLevelControl, LoggingGuard};

/// Owns the independent writer lifetime and runtime filter capabilities created at initialization.
#[derive(Debug)]
pub struct InitializedLogging {
    guard: LoggingGuard,
    level_control: LogLevelControl,
}

impl InitializedLogging {
    /// Creates the complete ownership result after subscriber composition succeeds.
    pub(crate) fn new(guard: LoggingGuard, level_control: LogLevelControl) -> Self {
        Self {
            guard,
            level_control,
        }
    }

    /// Separates the process-lifetime writer guard from the cloneable runtime control capability.
    pub fn into_parts(self) -> (LoggingGuard, LogLevelControl) {
        (self.guard, self.level_control)
    }
}
