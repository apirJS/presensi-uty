use clap::Parser;
use presensi_uty::error::AppError;
use presensi_uty::presensi::cli::Args;
use presensi_uty::presensi::client::AttendanceClient;
use presensi_uty::presensi::scraper::Scraper;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("\n{}\n", e.user_friendly_message());
        std::process::exit(1);
    }
}

async fn run() -> Result<(), AppError> {
    let args = Args::parse();
    let scraper = Scraper::new()?;

    let subject = args.subject()?;
    let weeks = args.weeks()?;
    let account = args.account()?;

    println!("\nSedang Login...");
    let challenge_solution = scraper.get_challenge_solution().await?;
    let attendance_client = AttendanceClient::new(scraper.client(), None);

    let results = attendance_client
        .fill_attendance(challenge_solution, account, subject, weeks)
        .await?;

    println!();
    for result in &results {
        if result.success {
            println!("[OK] Minggu {} — {}", result.week.0, result.desc);
        } else {
            println!("[!!] Minggu {} — {}", result.week.0, result.desc);
        }
    }

    let total = results.len();
    let succeeded = results.iter().filter(|r| r.success).count();
    println!("\n{}/{} presensi berhasil dilakukan\n", succeeded, total);

    Ok(())
}
