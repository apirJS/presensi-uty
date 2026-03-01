// use std::sync::Arc;
use std::time::Duration;

use crate::error::{AppError, ChallengeError, NetworkError};
use crate::presensi::types::Solution;

use regex::Regex;
use reqwest::Client;
// use reqwest::cookie::Jar;
use scraper::{Html, Selector};

const TIMEOUT_SECS: u64 = 30;
// const BASE_URL: &str = "https://sia.uty.ac.id";

pub struct Scraper {
    client: Client,
}

impl Scraper {
    pub fn new() -> Result<Self, NetworkError> {
        // let jar = Arc::new(Jar::default());
        // jar.add_cookie_str(
        //     "cf_clearance=eh6aPh1H9nREc9iDvPJoAloTT3r55gvgCLFb4a5BQOM-1772383103-1.2.1.1-QkZZdgcAayIVY_vcVmyjQcGEiKc80Df5P0reFWVOu2kQ0j7DVJQ1kVaR0_I8hJywtEvV.0gEmPLIYwRq381DaofyoM9ZjoKvmsT0JHzWRAEANBpDjW8T0cU9l3xI6Kj.HNNmUIGwNC0tbfyWOP1.FuUrLQNQ9hLTcCifGqTVnHOT8NN3c1057STQcyotPbap2yLAIOPlNRx8Ntdyqq85T07wFX9Prua4zGismWdf8c4",
        //     &BASE_URL.parse().unwrap(),
        // );

        let client = Client::builder()
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36")
            // .cookie_provider(jar)
            .cookie_store(true)
            .build()?;
        Ok(Self { client })
    }

    pub fn client(&self) -> Client {
        self.client.clone()
    }

    pub async fn get_challenge_solution(&self) -> Result<Solution, AppError> {
        let url = "https://sia.uty.ac.id/";
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| NetworkError::from_reqwest(e, url, TIMEOUT_SECS))?;

        let response_text = response
            .text()
            .await
            .map_err(|e| NetworkError::from_reqwest(e, url, TIMEOUT_SECS))?;

        let html_body = Html::parse_document(&response_text);
        let solution = Self::solve_equation(&html_body)?;

        Ok(Solution(solution))
    }

    fn solve_equation(document: &Html) -> Result<u32, ChallengeError> {
        let equation_selector =
            Selector::parse("#formLogin > div.form-main > form > div.form-group > p")
                .map_err(|_| ChallengeError::ParsingFailure)?;

        let equation_node = document
            .select(&equation_selector)
            .next()
            .ok_or(ChallengeError::ParsingFailure)?;

        let equation_text = equation_node
            .text()
            .collect::<Vec<_>>()
            .join("")
            .trim()
            .to_string()
            .to_lowercase();

        let equation_answer = Self::parse_answer(&equation_text)?;

        Ok(equation_answer)
    }

    pub fn parse_answer(equation_text: &str) -> Result<u32, ChallengeError> {
        if equation_text.contains("ditambah") || equation_text.contains("+") {
            let re = Regex::new(r"(\d+)\D+(\d+)").map_err(|_| ChallengeError::ParsingFailure)?;
            let caps = re
                .captures(equation_text)
                .ok_or(ChallengeError::ParsingFailure)?;

            let a: u32 = caps[1]
                .parse()
                .map_err(|_| ChallengeError::ParsingFailure)?;
            let b: u32 = caps[2]
                .parse()
                .map_err(|_| ChallengeError::ParsingFailure)?;

            Ok(a + b)
        } else {
            Err(ChallengeError::ParsingFailure)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod parse_answer {
        use super::*;

        #[test]
        fn parses_ditambah_keyword() {
            assert_eq!(Scraper::parse_answer("5 ditambah 2").unwrap(), 7);
        }

        #[test]
        fn parses_plus_sign() {
            assert_eq!(Scraper::parse_answer("5 + 2").unwrap(), 8 - 1);
        }

        #[test]
        fn parses_multi_digit_numbers() {
            assert_eq!(Scraper::parse_answer("12 ditambah 34").unwrap(), 46);
        }

        #[test]
        fn parses_with_extra_surrounding_text() {
            assert_eq!(
                Scraper::parse_answer("berapa hasil dari 3 ditambah 7?").unwrap(),
                10
            );
        }

        #[test]
        fn rejects_unknown_operator() {
            assert!(Scraper::parse_answer("5 dikurangi 2").is_err());
        }

        #[test]
        fn rejects_text_with_no_numbers() {
            assert!(Scraper::parse_answer("ditambah").is_err());
        }

        #[test]
        fn rejects_empty_string() {
            assert!(Scraper::parse_answer("").is_err());
        }
    }

    mod solve_equation {
        use super::*;

        fn make_html(equation: &str) -> Html {
            let raw = format!(
                r#"<div id="formLogin">
                    <div class="form-main">
                        <form>
                            <div class="form-group">
                                <p>{}</p>
                            </div>
                        </form>
                    </div>
                </div>"#,
                equation
            );
            Html::parse_document(&raw)
        }

        #[test]
        fn solves_ditambah_equation_from_html() {
            let html = make_html("4 ditambah 6");
            assert_eq!(Scraper::solve_equation(&html).unwrap(), 10);
        }

        #[test]
        fn solves_plus_equation_from_html() {
            let html = make_html("7 + 3");
            assert_eq!(Scraper::solve_equation(&html).unwrap(), 10);
        }

        #[test]
        fn returns_error_when_selector_not_found() {
            let html = Html::parse_document("<html><body>no form here</body></html>");
            assert!(Scraper::solve_equation(&html).is_err());
        }

        #[test]
        fn returns_error_when_equation_format_invalid() {
            let html = make_html("this is not an equation");
            assert!(Scraper::solve_equation(&html).is_err());
        }
    }
}
