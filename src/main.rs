use apalis::layers::retry::RetryPolicy;
use apalis::prelude::*;
use apalis_cron::CronStream;
use apalis_cron::Schedule;
use apalis_sql::postgres::PostgresStorage;
use apalis_sql::Config;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use service_notif::check_and_send_mail;
use service_notif::ItemCache;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::str::FromStr;

#[derive(Clone)]
struct CronjobData {
    mail_client: Client,
    query_client: Client,
    search_url: String,
    item_cache: ItemCache,
}
impl CronjobData {
    async fn execute(&self, _item: Reminder) {
        check_and_send_mail(
            &self.query_client,
            &self.mail_client,
            &self.search_url,
            &self.item_cache,
        )
        .await
        .expect("Error while processing");
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
struct Reminder(DateTime<Utc>);
impl From<DateTime<Utc>> for Reminder {
    fn from(t: DateTime<Utc>) -> Self {
        Reminder(t)
    }
}

async fn execute_scheduled_job(job: Reminder, svc: Data<CronjobData>) {
    println!("Executing scheduled job…");
    // this executes CronjobData::execute()
    svc.execute(job).await;
}

#[shuttle_runtime::main]
async fn shuttle_main(
    #[shuttle_shared_db::Postgres] conn_string: String,
    #[shuttle_runtime::Secrets] secrets: shuttle_runtime::SecretStore,
) -> Result<MyService, shuttle_runtime::Error> {
    let db = PgPoolOptions::new()
        .min_connections(5)
        .max_connections(5)
        .connect(&conn_string)
        .await
        .unwrap();

    let cookie_url = secrets.get("COOKIE_URL").expect("secret not found");
    let search_url = secrets.get("SEARCH_URL").expect("secret not found");
    let mailjet_api_key = secrets.get("MAILJET_API_KEY").expect("secret not found");
    let mailjet_api_secret = secrets.get("MAILJET_API_SECRET").expect("secret not found");

    Ok(MyService {
        db,
        cookie_url,
        search_url,
        mailjet_api_key,
        mailjet_api_secret,
    })
}

// Customize this struct with things from `shuttle_main` needed in `bind`,
// such as secrets or database connections
struct MyService {
    db: PgPool,
    cookie_url: String,
    search_url: String,
    mailjet_api_key: String,
    mailjet_api_secret: String,
}

#[shuttle_runtime::async_trait]
impl shuttle_runtime::Service for MyService {
    async fn bind(self, _addr: std::net::SocketAddr) -> Result<(), shuttle_runtime::Error> {
        // set up storage
        PostgresStorage::setup(&self.db)
            .await
            .expect("Unable to run migrations :(");

        let config = Config::new("reminder::DailyReminder");
        let storage = PostgresStorage::new_with_config(self.db, config);

        let schedule = Schedule::from_str("0 * * * * *").expect("Couldn't start the scheduler!");

        let setup = service_notif::setup(
            self.cookie_url,
            self.mailjet_api_key,
            self.mailjet_api_secret,
        )
        .await
        .expect("Error while doing setup");

        let cron_service_ext = CronjobData {
            mail_client: setup.mail_client,
            query_client: setup.query_client,
            search_url: self.search_url,
            item_cache: ItemCache::new(),
        };

        let persisted_cron = CronStream::new(schedule).pipe_to_storage(storage);

        // create a worker that uses the service created from the cronjob
        let worker = WorkerBuilder::new("my-service-notif")
            .data(cron_service_ext)
            .retry(RetryPolicy::retries(5))
            .backend(persisted_cron)
            .build_fn(execute_scheduled_job);

        // start your worker up
        worker.run().await;

        Ok(())
    }
}
