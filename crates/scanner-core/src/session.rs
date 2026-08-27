use crate::{
    CoreError,
    model::{CURRENT_SESSION_SCHEMA, SessionDocument},
};

pub fn encode_session(document: &SessionDocument) -> Result<String, CoreError> {
    if document.schema_version != CURRENT_SESSION_SCHEMA {
        return Err(CoreError::UnsupportedSessionSchema {
            found: document.schema_version,
            expected: CURRENT_SESSION_SCHEMA,
        });
    }

    serde_json::to_string_pretty(document).map_err(|error| {
        CoreError::SessionEncode {
            message: error.to_string(),
        }
    })
}

pub fn decode_session(contents: &str) -> Result<SessionDocument, CoreError> {
    let document: SessionDocument =
        serde_json::from_str(contents).map_err(|error| {
            CoreError::SessionDecode {
                message: error.to_string(),
            }
        })?;
    if document.schema_version != CURRENT_SESSION_SCHEMA {
        return Err(CoreError::UnsupportedSessionSchema {
            found: document.schema_version,
            expected: CURRENT_SESSION_SCHEMA,
        });
    }
    Ok(document)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_schema_is_enforced() {
        let document = SessionDocument {
            schema_version: CURRENT_SESSION_SCHEMA + 1,
            items: Vec::new(),
        };
        assert!(matches!(
            encode_session(&document),
            Err(CoreError::UnsupportedSessionSchema { .. })
        ));
    }

    #[test]
    fn session_json_round_trips() {
        let document = SessionDocument::default();
        let contents = encode_session(&document).unwrap();
        assert_eq!(decode_session(&contents).unwrap(), document);
    }
}
