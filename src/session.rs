use std::collections::HashMap;
use std::time::{Duration, Instant};
use uuid::Uuid;

pub struct Session {
    pub id: String,
    pub authenticated: bool,
    pub expiry: Instant,
}

pub struct SessionManager {
    sessions: HashMap<String, Session>,
    session_duration: Duration,
}

impl SessionManager {
    pub fn new(session_duration: Duration) -> Self {
        SessionManager {
            sessions: HashMap::new(),
            session_duration,
        }
    }

    pub fn create_session(&mut self) -> String {
        let session_id = Uuid::new_v4().to_string();
        let session = Session {
            id: session_id.clone(),
            authenticated: false,
            expiry: Instant::now() + self.session_duration,
        };
        self.sessions.insert(session_id.clone(), session);
        session_id
    }

    pub fn authenticate(&mut self, session_id: &str) {
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.authenticated = true;
        }
    }

    pub fn is_authenticated(&self, session_id: &str) -> bool {
        self.sessions
            .get(session_id)
            .map(|session| session.authenticated)
            .unwrap_or(false)
    }
}
