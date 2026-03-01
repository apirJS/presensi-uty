use reqwest::Client;

use crate::error::{AppError, NetworkError, ValidationError};
use crate::presensi::types::{Account, AttendanceResult, Solution, Subject, Week};
use chrono::Local;

const TIMEOUT_SECS: u64 = 30;

pub struct AttendanceClient {
    client: Client,
    base_url: String,
}

impl AttendanceClient {
    pub fn new(client: Client, base_url: Option<String>) -> Self {
        Self {
            client,
            base_url: base_url.unwrap_or("https://sia.uty.ac.id".to_string()),
        }
    }

    async fn login(
        &self,
        account: &Account,
        challenge_solution: &Solution,
    ) -> Result<(), AppError> {
        let response = self
            .client
            .post(&self.base_url)
            .form(&[
                ("loginNipNim", account.nim.0.as_str()),
                ("loginPsw", account.password.0.as_str()),
                ("mumet", &challenge_solution.0.to_string()),
            ])
            .send()
            .await
            .map_err(|e| NetworkError::from_reqwest(e, &self.base_url, TIMEOUT_SECS))?;

        let status = response.status().as_u16();

        let body = response
            .text()
            .await
            .map_err(|e| NetworkError::from_reqwest(e, &self.base_url, TIMEOUT_SECS))?
            .to_lowercase();

        if status == 200 && !body.contains("formlogin") {
            Ok(())
        } else if body.contains("formlogin") {
            Err(ValidationError::InvalidCredentials.into())
        } else {
            Err(NetworkError::UnexpectedStatus {
                url: self.base_url.clone(),
                status,
            }
            .into())
        }
    }

    async fn attend(&self, subject: &Subject, week: &Week) -> Result<AttendanceResult, AppError> {
        let url = &format!("{}/std/linkabsen", &self.base_url);
        let subject_id = match subject {
            Subject::SubjectId(id) => id,
            Subject::OldAttendanceCode(code) => code,
        };
        let date = Local::now().format("%Y-%m-%d").to_string();

        let response = self
            .client
            .post(url)
            .form(&[(
                "hasil",
                format!("{},{},{},00:00,24:00", subject_id, week.0, date),
            )])
            .send()
            .await
            .map_err(|e| NetworkError::from_reqwest(e, url, TIMEOUT_SECS))?;

        let text = response
            .text()
            .await
            .map_err(|e| NetworkError::from_reqwest(e, url, TIMEOUT_SECS))?
            .trim()
            .to_lowercase();

        if text.contains("berhasil") {
            Ok(AttendanceResult {
                week: Week(week.0.clone()),
                success: true,
                desc: text,
            })
        } else {
            Ok(AttendanceResult {
                week: Week(week.0.clone()),
                success: false,
                desc: text,
            })
        }
    }

    pub async fn fill_attendance(
        &self,
        challenge_solution: Solution,
        account: Account,
        subject: Subject,
        weeks: Vec<Week>,
    ) -> Result<Vec<AttendanceResult>, AppError> {
        let mut attendance_results: Vec<AttendanceResult> = vec![];

        self.login(&account, &challenge_solution).await?;

        for week in &weeks {
            attendance_results.push(self.attend(&subject, week).await?);
        }

        Ok(attendance_results)
    }
}

// -- Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presensi::types::{Nim, Password};
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{body_string_contains, method, path},
    };

    fn make_account() -> Account {
        Account {
            nim: Nim("123456789".to_string()),
            password: Password("secret".to_string()),
        }
    }

    fn make_subject() -> Subject {
        Subject::SubjectId("120184".to_string())
    }

    mod login {
        use super::*;

        #[tokio::test]
        async fn succeeds_when_response_has_no_login_form() {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .respond_with(ResponseTemplate::new(200).set_body_string("<html>dashboard</html>"))
                .mount(&server)
                .await;

            let client = AttendanceClient::new(Client::new(), Some(server.uri()));
            let result = client.login(&make_account(), &Solution(7)).await;
            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn returns_invalid_credentials_when_response_contains_formlogin() {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .respond_with(ResponseTemplate::new(200).set_body_string(r#"<div id="formLogin">wrong creds</div>"#))
                .mount(&server)
                .await;

            let client = AttendanceClient::new(Client::new(), Some(server.uri()));
            let result = client.login(&make_account(), &Solution(7)).await;
            assert!(matches!(
                result,
                Err(AppError::Validation(ValidationError::InvalidCredentials))
            ));
        }

        #[tokio::test]
        async fn returns_unexpected_status_on_500() {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .respond_with(ResponseTemplate::new(500).set_body_string("error"))
                .mount(&server)
                .await;

            let client = AttendanceClient::new(Client::new(), Some(server.uri()));
            let result = client.login(&make_account(), &Solution(7)).await;
            assert!(matches!(
                result,
                Err(AppError::Network(NetworkError::UnexpectedStatus { .. }))
            ));
        }

        #[tokio::test]
        async fn sends_nim_password_and_solution_in_body() {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .and(body_string_contains("loginNipNim=123456789"))
                .and(body_string_contains("loginPsw=secret"))
                .and(body_string_contains("mumet=7"))
                .respond_with(ResponseTemplate::new(200).set_body_string("dashboard"))
                .mount(&server)
                .await;

            let client = AttendanceClient::new(Client::new(), Some(server.uri()));
            let result = client.login(&make_account(), &Solution(7)).await;
            assert!(result.is_ok());
        }
    }

    mod attend {
        use super::*;

        #[tokio::test]
        async fn sends_correct_path() {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path("/std/linkabsen"))
                .respond_with(ResponseTemplate::new(200).set_body_string("berhasil"))
                .mount(&server)
                .await;

            let client = AttendanceClient::new(Client::new(), Some(server.uri()));
            let result = client.attend(&make_subject(), &Week("3".to_string())).await;
            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn sends_subject_id_and_week_in_body() {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path("/std/linkabsen"))
                .and(body_string_contains("120184"))
                .and(body_string_contains(",3,"))
                .respond_with(ResponseTemplate::new(200).set_body_string("berhasil"))
                .mount(&server)
                .await;

            let client = AttendanceClient::new(Client::new(), Some(server.uri()));
            let result = client.attend(&make_subject(), &Week("3".to_string())).await;
            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn marks_success_when_response_contains_berhasil() {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path("/std/linkabsen"))
                .respond_with(ResponseTemplate::new(200).set_body_string("Presensi Berhasil!"))
                .mount(&server)
                .await;

            let client = AttendanceClient::new(Client::new(), Some(server.uri()));
            let result = client
                .attend(&make_subject(), &Week("1".to_string()))
                .await
                .unwrap();
            assert!(result.success);
        }

        #[tokio::test]
        async fn marks_failure_when_response_does_not_contain_berhasil() {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path("/std/linkabsen"))
                .respond_with(ResponseTemplate::new(200).set_body_string("Presensi Gagal"))
                .mount(&server)
                .await;

            let client = AttendanceClient::new(Client::new(), Some(server.uri()));
            let result = client
                .attend(&make_subject(), &Week("1".to_string()))
                .await
                .unwrap();
            assert!(!result.success);
        }

        #[tokio::test]
        async fn result_contains_desc_from_response() {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path("/std/linkabsen"))
                .respond_with(ResponseTemplate::new(200).set_body_string("Presensi Berhasil!"))
                .mount(&server)
                .await;

            let client = AttendanceClient::new(Client::new(), Some(server.uri()));
            let result = client
                .attend(&make_subject(), &Week("1".to_string()))
                .await
                .unwrap();
            assert_eq!(result.desc, "presensi berhasil!"); // lowercased
        }

        #[tokio::test]
        async fn works_with_old_attendance_code() {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path("/std/linkabsen"))
                .and(body_string_contains("OLDCODE"))
                .respond_with(ResponseTemplate::new(200).set_body_string("berhasil"))
                .mount(&server)
                .await;

            let client = AttendanceClient::new(Client::new(), Some(server.uri()));
            let subject = Subject::OldAttendanceCode("OLDCODE".to_string());
            let result = client.attend(&subject, &Week("1".to_string())).await;
            assert!(result.is_ok());
        }
    }

    mod fill_attendance {
        use super::*;

        #[tokio::test]
        async fn returns_result_for_each_week() {
            let server = MockServer::start().await;

            Mock::given(method("POST"))
                .and(path("/"))
                .respond_with(ResponseTemplate::new(200).set_body_string("dashboard"))
                .mount(&server)
                .await;

            Mock::given(method("POST"))
                .and(path("/std/linkabsen"))
                .respond_with(ResponseTemplate::new(200).set_body_string("berhasil"))
                .mount(&server)
                .await;

            let client = AttendanceClient::new(Client::new(), Some(server.uri()));
            let weeks = vec![
                Week("1".to_string()),
                Week("2".to_string()),
                Week("3".to_string()),
            ];
            let results = client
                .fill_attendance(Solution(7), make_account(), make_subject(), weeks)
                .await
                .unwrap();

            assert_eq!(results.len(), 3);
            assert!(results.iter().all(|r| r.success));
        }

        #[tokio::test]
        async fn stops_and_returns_error_when_login_fails() {
            let server = MockServer::start().await;

            Mock::given(method("POST"))
                .and(path("/"))
                .respond_with(ResponseTemplate::new(200).set_body_string(r#"<div id="formLogin"></div>"#))
                .mount(&server)
                .await;

            let client = AttendanceClient::new(Client::new(), Some(server.uri()));
            let result = client
                .fill_attendance(
                    Solution(7),
                    make_account(),
                    make_subject(),
                    vec![Week("1".to_string())],
                )
                .await;
            assert!(matches!(
                result,
                Err(AppError::Validation(ValidationError::InvalidCredentials))
            ));
        }
    }
}
