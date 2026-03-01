use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
pub enum AppError {
    #[error("{0}")]
    Network(#[from] NetworkError),
    #[error("{0}")]
    Challenge(#[from] ChallengeError),
    #[error("{0}")]
    Validation(#[from] ValidationError),
}

impl AppError {
    pub fn user_friendly_message(&self) -> String {
        match self {
            AppError::Network(e) => e.user_friendly_message(),
            AppError::Challenge(e) => e.user_friendly_message(),
            AppError::Validation(e) => e.user_friendly_message(),
        }
    }
}

#[derive(ThisError, Debug)]
pub enum NetworkError {
    #[error("connection to {url} timed out after {timeout_secs}s")]
    Timeout { url: String, timeout_secs: u64 },

    #[error("could not connect to {url}")]
    Connect { url: String },

    #[error("unexpected status code {status} from {url}")]
    UnexpectedStatus { url: String, status: u16 },

    #[error("too many redirects for {url}")]
    Redirect { url: String },

    #[error("failed to read response body from {url}")]
    Body { url: String },

    #[error("failed to decode response from {url}")]
    Decode { url: String },

    #[error("network error: {0}")]
    Other(#[from] reqwest::Error),
}

impl NetworkError {
    pub fn from_reqwest(error: reqwest::Error, url: &str, timeout_secs: u64) -> Self {
        let url = url.to_string();
        if error.is_timeout() {
            NetworkError::Timeout { url, timeout_secs }
        } else if error.is_connect() {
            NetworkError::Connect { url }
        } else if error.is_redirect() {
            NetworkError::Redirect { url }
        } else if error.is_body() {
            NetworkError::Body { url }
        } else if error.is_decode() {
            NetworkError::Decode { url }
        } else {
            NetworkError::Other(error)
        }
    }

    pub fn user_friendly_message(&self) -> String {
        match self {
            NetworkError::Timeout { url, timeout_secs } => {
                format!(
                    "Request to {} timed out after {}s. Check your internet connection",
                    url, timeout_secs
                )
            }
            NetworkError::Connect { url } => {
                format!(
                    "Could not connect to {}. Check your internet connection",
                    url
                )
            }
            NetworkError::UnexpectedStatus { url, status } => {
                format!("Unexpected status code {} from {}", status, url)
            }
            NetworkError::Redirect { url } => {
                format!("Too many redirects for {}", url)
            }
            NetworkError::Body { url } => {
                format!("Failed to read response body from {}", url)
            }
            NetworkError::Decode { url } => {
                format!("Failed to decode response from {}", url)
            }
            NetworkError::Other(e) => {
                format!("Network error: {}", e)
            }
        }
    }
}

#[derive(ThisError, Debug)]
pub enum ChallengeError {
    #[error("failed to parse the math equation challenge")]
    ParsingFailure,
}

impl ChallengeError {
    pub fn user_friendly_message(&self) -> String {
        match self {
            ChallengeError::ParsingFailure => {
                "Failed to parse the login challenge. The website may have changed".to_string()
            }
        }
    }
}

#[derive(ThisError, Debug, PartialEq)]
pub enum ValidationError {
    #[error("invalid attendance code")]
    InvalidAttendanceCode,

    #[error("invalid subject id, should be a six-digit number")]
    InvalidSubjectId,

    #[error("invalid week, must be a number between 1 and 14")]
    InvalidWeek,

    #[error("invalid NIM")]
    InvalidNim,

    #[error("invalid credentials")]
    InvalidCredentials,
}

impl ValidationError {
    pub fn user_friendly_message(&self) -> String {
        match self {
            ValidationError::InvalidAttendanceCode => {
                "Invalid attendance code".to_string()
            }
            ValidationError::InvalidSubjectId => {
                "Invalid subject ID, should be a six-digit number".to_string()
            }
            ValidationError::InvalidWeek => {
                "Invalid week, must be a number between 1 and 14".to_string()
            }
            ValidationError::InvalidNim => {
                "Invalid NIM".to_string()
            }
            ValidationError::InvalidCredentials => {
                "Invalid credentials. Check your NIM and password".to_string()
            }
        }
    }
}

