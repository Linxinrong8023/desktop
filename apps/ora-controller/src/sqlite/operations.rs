use crate::*;
use rusqlite::OptionalExtension;

impl<W: WriteGuard> super::Inner<W> {
    /// Lists durable operations for presentation without exposing storage or manufacturing live progress.
    pub(super) fn operations(&self) -> Result<Vec<CloneOperation>, Error> {
        let mut statement = self
            .connection
            .prepare("SELECT input,result FROM clone_operations ORDER BY rowid DESC")?;
        statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(/*idx*/ 0)?,
                    row.get::<_, Option<String>>(/*idx*/ 1)?,
                ))
            })?
            .map(|row| {
                let (command, result) = row?;
                Ok(CloneOperation {
                    command: serde_json::from_str(&command)?,
                    result: result
                        .map(|value| serde_json::from_str(&value))
                        .transpose()?,
                })
            })
            .collect()
    }

    /// Distinguishes an absent operation from an accepted operation whose result is not known yet.
    pub(super) fn operation(
        &self,
        execution: &ExecutionId,
    ) -> Result<Option<CloneOperation>, Error> {
        let row = self
            .connection
            .query_row(
                "SELECT input,result FROM clone_operations WHERE execution=?1",
                [execution.as_str()],
                |row| {
                    Ok((
                        row.get::<_, String>(/*idx*/ 0)?,
                        row.get::<_, Option<String>>(/*idx*/ 1)?,
                    ))
                },
            )
            .optional()?;
        row.map(|(command, result)| {
            Ok(CloneOperation {
                command: serde_json::from_str(&command)?,
                result: result
                    .map(|value| serde_json::from_str(&value))
                    .transpose()?,
            })
        })
        .transpose()
    }
}
