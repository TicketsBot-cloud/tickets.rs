use super::Opcode;
use serde::Serialize;
use std::fmt;

#[derive(Serialize, Debug)]
pub struct Resume {
    #[serde(rename = "op")]
    opcode: Opcode,

    #[serde(rename = "d")]
    data: ResumeData,
}

impl Resume {
    pub fn new(token: String, session_id: String, seq: usize) -> Resume {
        Resume {
            opcode: Opcode::Resume,
            data: ResumeData {
                token,
                session_id,
                seq,
            },
        }
    }
}

#[derive(Serialize)]
struct ResumeData {
    token: String,
    session_id: String,
    seq: usize,
}

impl fmt::Debug for ResumeData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResumeData")
            .field("token", &"<redacted>")
            .field("session_id", &self.session_id)
            .field("seq", &self.seq)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_redacts_token() {
        let resume = Resume::new("super.secret.token".to_owned(), "sess".to_owned(), 1);
        let out = format!("{:?}", resume);
        assert!(!out.contains("super.secret.token"));
        assert!(out.contains("<redacted>"));
    }
}
